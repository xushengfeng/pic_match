use rustfft::{num_complex::Complex, FftPlanner};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static PLANNER: RefCell<FftPlanner<f64>> = RefCell::new(FftPlanner::new());
}

/// 小图降采样后的最短边下限。低于这个尺寸，相位相关的峰值会被噪声淹没。
const MIN_WORK_DIM: usize = 32;
/// 单次 FFT 的边长上限，用来约束耗时与内存。
const PAD_LIMIT: usize = 1024;
/// Tukey 窗两端的渐变比例。越小越接近矩形窗，对部分重叠越友好。
const TAPER: f64 = 0.1;
/// 计算 PSR 时排除的峰值邻域半径。
const PSR_EXCLUDE: isize = 5;
/// PSR 上限：完全重合时旁瓣方差为 0，用它表示"完美匹配"。
const PSR_MAX: f64 = 100.0;

fn next_power_of_2(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

fn rgba_to_grayscale(pixels: &[u8], width: usize, height: usize) -> Vec<f64> {
    let mut gray = vec![0.0f64; width * height];
    for i in 0..width * height {
        let r = pixels[i * 4] as f64;
        let g = pixels[i * 4 + 1] as f64;
        let b = pixels[i * 4 + 2] as f64;
        gray[i] = 0.299 * r + 0.587 * g + 0.114 * b;
    }
    gray
}

fn downsample(gray: &[f64], w: usize, h: usize, factor: usize) -> (Vec<f64>, usize, usize) {
    if factor <= 1 {
        return (gray.to_vec(), w, h);
    }
    let nw = w / factor;
    let nh = h / factor;
    let mut out = vec![0.0f64; nw * nh];
    let f2 = (factor * factor) as f64;
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = 0.0;
            let sy = y * factor;
            let sx = x * factor;
            for dy in 0..factor {
                for dx in 0..factor {
                    sum += gray[(sy + dy) * w + (sx + dx)];
                }
            }
            out[y * nw + x] = sum / f2;
        }
    }
    (out, nw, nh)
}

fn padded_len(a: usize, b: usize, factor: usize) -> usize {
    let s = a / factor + b / factor;
    next_power_of_2(s.max(2))
}

/// 选降采样倍数：保证小图仍有足够像素，同时用边长上限约束耗时。
fn choose_factor(wa: usize, ha: usize, wb: usize, hb: usize) -> usize {
    let min_dim = wa.min(ha).min(wb).min(hb).max(1);
    let mut factor = 1usize;
    while factor < 16
        && min_dim / factor > MIN_WORK_DIM
        && (padded_len(wa, wb, factor) > PAD_LIMIT
            || padded_len(ha, hb, factor) > PAD_LIMIT)
    {
        factor *= 2;
    }
    factor
}

/// Tukey 窗：中间平坦、两端余弦渐变，相对 Hanning 窗保留更多有效面积。
fn tukey(n: usize, alpha: f64) -> Vec<f64> {
    let mut win = vec![1.0f64; n];
    if n <= 1 || alpha <= 0.0 {
        return win;
    }
    let alpha = alpha.min(1.0);
    let half = alpha * (n - 1) as f64 / 2.0;
    if half < 1.0 {
        return win;
    }
    for (i, w) in win.iter_mut().enumerate() {
        let d = (i as f64).min((n - 1) as f64 - i as f64);
        if d < half {
            *w = 0.5 * (1.0 + (std::f64::consts::PI * (d / half - 1.0)).cos());
        }
    }
    win
}

/// 去均值 + 加窗，降低直流分量和边界不连续带来的泄漏。
fn prepare(gray: &[f64], w: usize, h: usize, alpha: f64) -> Vec<f64> {
    let mut out = gray.to_vec();
    let mean = if out.is_empty() {
        0.0
    } else {
        out.iter().sum::<f64>() / out.len() as f64
    };
    let wx = tukey(w, alpha);
    let wy = tukey(h, alpha);
    for y in 0..h {
        let wyv = wy[y];
        for x in 0..w {
            out[y * w + x] = (out[y * w + x] - mean) * wx[x] * wyv;
        }
    }
    out
}

fn fft_row(row: &mut [Complex<f64>], inverse: bool) {
    let n = row.len();
    PLANNER.with(|planner| {
        let fft = if inverse {
            planner.borrow_mut().plan_fft_inverse(n)
        } else {
            planner.borrow_mut().plan_fft_forward(n)
        };
        fft.process(row);
    });
}

fn fft_2d(data: &mut [Complex<f64>], width: usize, height: usize, inverse: bool) {
    let mut row = vec![Complex::new(0.0, 0.0); width];
    for y in 0..height {
        let start = y * width;
        row.copy_from_slice(&data[start..start + width]);
        fft_row(&mut row, inverse);
        data[start..start + width].copy_from_slice(&row);
    }

    let mut col = vec![Complex::new(0.0, 0.0); height];
    for x in 0..width {
        for y in 0..height {
            col[y] = data[y * width + x];
        }
        fft_row(&mut col, inverse);
        for y in 0..height {
            data[y * width + x] = col[y];
        }
    }

    if inverse {
        let n = (width * height) as f64;
        for v in data.iter_mut() {
            v.re /= n;
            v.im /= n;
        }
    }
}

/// 一张灰度图（或降采样后的灰度图）
#[derive(Clone, Copy)]
struct Frame<'a> {
    data: &'a [f64],
    w: usize,
    h: usize,
}

fn wrap(mut v: isize, n: isize) -> usize {
    while v < 0 {
        v += n;
    }
    while v >= n {
        v -= n;
    }
    v as usize
}

/// 一维抛物线插值，返回 [-0.5, 0.5] 内的亚像素修正量。
fn subpixel(prev: f64, cur: f64, next: f64) -> f64 {
    let denom = prev - 2.0 * cur + next;
    if denom >= 0.0 {
        return 0.0;
    }
    let delta = 0.5 * (prev - next) / denom;
    if delta.abs() <= 0.5 {
        delta
    } else {
        0.0
    }
}

struct Correlation {
    dx: f64,
    dy: f64,
    psr: f64,
    peak: f64,
    ncc: f64,
    overlap: f64,
}

/// 相位相关峰的位置与高度（整数索引）
#[derive(Clone)]
struct Candidate {
    i: usize,
    j: usize,
    value: f64,
}

fn correlate(a: Frame, b: Frame) -> Correlation {
    let (wa, ha, wb, hb) = (a.w, a.h, b.w, b.h);
    let pad_w = next_power_of_2(wa + wb);
    let pad_h = next_power_of_2(ha + hb);
    let n = pad_w * pad_h;

    let a_win = prepare(a.data, wa, ha, TAPER);
    let b_win = prepare(b.data, wb, hb, TAPER);

    let mut fa = vec![Complex::new(0.0, 0.0); n];
    let mut fb = vec![Complex::new(0.0, 0.0); n];

    for y in 0..ha {
        for x in 0..wa {
            fa[y * pad_w + x] = Complex::new(a_win[y * wa + x], 0.0);
        }
    }
    for y in 0..hb {
        for x in 0..wb {
            fb[y * pad_w + x] = Complex::new(b_win[y * wb + x], 0.0);
        }
    }

    fft_2d(&mut fa, pad_w, pad_h, false);
    fft_2d(&mut fb, pad_w, pad_h, false);

    // 互功率谱归一化（PHAT），同时清理不再需要的 fb，降低峰值内存
    for i in 0..n {
        let conj = Complex::new(fa[i].re, -fa[i].im);
        let product = conj * fb[i];
        let mag = (product.re * product.re + product.im * product.im).sqrt();
        fa[i] = if mag > 1e-10 {
            Complex::new(product.re / mag, product.im / mag)
        } else {
            Complex::new(0.0, 0.0)
        };
    }
    drop(fb);

    fft_2d(&mut fa, pad_w, pad_h, true);

    // 只在物理可达的滞后里找峰：k ∈ [-(wa-1), wb-1]
    let valid_x = |i: usize| i < wb || i >= pad_w - (wa - 1);
    let valid_y = |j: usize| j < hb || j >= pad_h - (ha - 1);

    let mut best = f64::NEG_INFINITY;
    let mut best_i = 0usize;
    let mut best_j = 0usize;
    for j in 0..pad_h {
        if !valid_y(j) {
            continue;
        }
        let row = j * pad_w;
        for i in 0..pad_w {
            if !valid_x(i) {
                continue;
            }
            let v = fa[row + i].re;
            if v > best {
                best = v;
                best_i = i;
                best_j = j;
            }
        }
    }

    // 整张相关面没有正值：纯色、全黑等没有可匹配内容的情况
    if !best.is_finite() || best <= 1e-12 {
        return Correlation {
            dx: 0.0,
            dy: 0.0,
            psr: 0.0,
            peak: 0.0,
            ncc: 0.0,
            overlap: 0.0,
        };
    }

    // PSR：峰值与旁瓣均值/标准差之比
    let r = PSR_EXCLUDE;
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    for v in fa.iter() {
        sum += v.re;
        sum_sq += v.re * v.re;
    }
    let mut count = n as f64;
    for dy in -r..=r {
        let y = wrap(best_j as isize + dy, pad_h as isize);
        for dx in -r..=r {
            let x = wrap(best_i as isize + dx, pad_w as isize);
            let v = fa[y * pad_w + x].re;
            sum -= v;
            sum_sq -= v * v;
            count -= 1.0;
        }
    }
    let psr = if count > 1.0 {
        let mean = sum / count;
        let var = (sum_sq / count - mean * mean).max(0.0);
        let std = var.sqrt();
        if best <= 1e-12 {
            0.0
        } else if std > 1e-12 {
            ((best - mean) / std).min(PSR_MAX)
        } else {
            PSR_MAX
        }
    } else {
        0.0
    };

    let lag_x = |i: usize| -> isize {
        if i < wb {
            i as isize
        } else {
            i as isize - pad_w as isize
        }
    };
    let lag_y = |j: usize| -> isize {
        if j < hb {
            j as isize
        } else {
            j as isize - pad_h as isize
        }
    };

    // 候选峰：把重叠区的 NCC 当作第二判据，避免低重叠时的伪峰
    let mut candidates = collect_candidates(&fa, pad_w, pad_h, &valid_x, &valid_y, best);
    if candidates.is_empty() {
        candidates.push(Candidate {
            i: best_i,
            j: best_j,
            value: best,
        });
    }

    let mut chosen = candidates[0].clone();
    let mut chosen_score = f64::NEG_INFINITY;
    let mut chosen_ncc = 0.0f64;
    let mut chosen_overlap = 0.0f64;
    for c in candidates.iter() {
        let dx = lag_x(c.i);
        let dy = lag_y(c.j);
        let (ncc, overlap) = verify_overlap(a, b, dx, dy);
        // 重叠越少，NCC 越容易偶然偏高，排序时按重叠量轻微打折
        let score = ncc * (overlap / 0.05).min(1.0).sqrt();
        if score > chosen_score {
            chosen_score = score;
            chosen = c.clone();
            chosen_ncc = ncc;
            chosen_overlap = overlap;
        }
    }

    let px = chosen.i as isize;
    let py = chosen.j as isize;
    let left = fa[wrap(py, pad_h as isize) * pad_w + wrap(px - 1, pad_w as isize)].re;
    let right = fa[wrap(py, pad_h as isize) * pad_w + wrap(px + 1, pad_w as isize)].re;
    let up = fa[wrap(py - 1, pad_h as isize) * pad_w + wrap(px, pad_w as isize)].re;
    let down = fa[wrap(py + 1, pad_h as isize) * pad_w + wrap(px, pad_w as isize)].re;

    let dx = lag_x(chosen.i) as f64 + subpixel(left, chosen.value, right);
    let dy = lag_y(chosen.j) as f64 + subpixel(up, chosen.value, down);

    Correlation {
        dx,
        dy,
        psr,
        peak: chosen.value,
        ncc: chosen_ncc,
        overlap: chosen_overlap,
    }
}

/// 取若干互不相邻的高峰，供 NCC 复核
fn collect_candidates(
    data: &[Complex<f64>],
    pad_w: usize,
    pad_h: usize,
    valid_x: &dyn Fn(usize) -> bool,
    valid_y: &dyn Fn(usize) -> bool,
    best: f64,
) -> Vec<Candidate> {
    const MIN_SEPARATION: isize = 8;
    const MAX_CANDIDATES: usize = 5;
    let floor = best * 0.5;

    let mut raw: Vec<Candidate> = Vec::new();
    for j in 0..pad_h {
        if !valid_y(j) {
            continue;
        }
        let row = j * pad_w;
        for i in 0..pad_w {
            if !valid_x(i) {
                continue;
            }
            let v = data[row + i].re;
            if v >= floor {
                raw.push(Candidate { i, j, value: v });
            }
        }
    }
    raw.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<Candidate> = Vec::new();
    for c in raw {
        if out.len() >= MAX_CANDIDATES {
            break;
        }
        let far = out.iter().all(|p| {
            let dx = wrap(p.i as isize - c.i as isize, pad_w as isize) as isize;
            let dy = wrap(p.j as isize - c.j as isize, pad_h as isize) as isize;
            let dx = dx.min(pad_w as isize - dx);
            let dy = dy.min(pad_h as isize - dy);
            dx > MIN_SEPARATION || dy > MIN_SEPARATION
        });
        if far {
            out.push(c);
        }
    }
    out
}

/// 在给定偏移下比较两张图的公共区域，返回 (NCC, 重叠面积占比)。
/// NCC ∈ [-1, 1]，只有像素真的对齐时才接近 1。
fn verify_overlap(a: Frame, b: Frame, dx: isize, dy: isize) -> (f64, f64) {
    let (wa, ha, wb, hb) = (a.w, a.h, b.w, b.h);
    // 相位相关的约定：a[x] 对应 b[x + dx]
    let x0 = (-dx).max(0).min(wa as isize) as usize;
    let x1 = (wb as isize - dx).max(0).min(wa as isize) as usize;
    let y0 = (-dy).max(0).min(ha as isize) as usize;
    let y1 = (hb as isize - dy).max(0).min(ha as isize) as usize;
    let area_a = (wa * ha) as f64;
    let area_b = (wb * hb) as f64;
    let smaller = area_a.min(area_b);

    if x1 <= x0 || y1 <= y0 {
        return (0.0, 0.0);
    }
    let ow = x1 - x0;
    let oh = y1 - y0;
    let cells = (ow * oh) as f64;

    // 重叠太小时 NCC 不可靠
    if cells < (smaller * 0.01).max(64.0) {
        return (0.0, cells / smaller);
    }

    let mut sa = 0.0f64;
    let mut sb = 0.0f64;
    let mut saa = 0.0f64;
    let mut sbb = 0.0f64;
    let mut sab = 0.0f64;
    for y in y0..y1 {
        let by = (y as isize + dy) as usize;
        let arow = y * wa;
        let brow = by * wb;
        for x in x0..x1 {
            let bx = (x as isize + dx) as usize;
            let va = a.data[arow + x];
            let vb = b.data[brow + bx];
            sa += va;
            sb += vb;
            saa += va * va;
            sbb += vb * vb;
            sab += va * vb;
        }
    }
    let n = cells;
    let cov = n * sab - sa * sb;
    let va = n * saa - sa * sa;
    let vb = n * sbb - sb * sb;

    // 任一边几乎无方差（纯色区域）时相似度无意义
    let va_floor = 1e-6 * (sa * sa).max(1.0);
    let vb_floor = 1e-6 * (sb * sb).max(1.0);
    let ncc = if va > va_floor && vb > vb_floor {
        (cov / (va * vb).sqrt()).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    (ncc, cells / smaller)
}

#[derive(serde::Serialize)]
pub struct MatchResult {
    pub dx: f64,
    pub dy: f64,
    /// 重叠区归一化互相关（NCC），匹配程度；1 表示完全一致，≥0.8 可信，<0.4 基本不可信。
    pub confidence: f64,
    /// 重叠面积占较小图的比例，0~1。
    pub overlap: f64,
    /// 相位相关的归一化峰值高度，用于调试。
    pub peak: f64,
    /// 峰值旁瓣比，用于调试。
    pub psr: f64,
}

/// 纯 Rust 入口，方便原生测试与复用。
pub fn match_offset(
    image_a: &[u8],
    width_a: usize,
    height_a: usize,
    image_b: &[u8],
    width_b: usize,
    height_b: usize,
) -> MatchResult {
    let gray_a_full = rgba_to_grayscale(image_a, width_a, height_a);
    let gray_b_full = rgba_to_grayscale(image_b, width_b, height_b);

    let factor = choose_factor(width_a, height_a, width_b, height_b);

    let (gray_a, wa, ha) = downsample(&gray_a_full, width_a, height_a, factor);
    let (gray_b, wb, hb) = downsample(&gray_b_full, width_b, height_b, factor);

    let corr = correlate(
        Frame {
            data: &gray_a,
            w: wa,
            h: ha,
        },
        Frame {
            data: &gray_b,
            w: wb,
            h: hb,
        },
    );
    let f = factor as f64;

    MatchResult {
        dx: corr.dx * f,
        dy: corr.dy * f,
        confidence: corr.ncc,
        overlap: corr.overlap,
        peak: corr.peak,
        psr: corr.psr,
    }
}

#[wasm_bindgen]
pub fn find_offset(
    image_a: &[u8],
    width_a: usize,
    height_a: usize,
    image_b: &[u8],
    width_b: usize,
    height_b: usize,
) -> JsValue {
    let result = match_offset(
        image_a, width_a, height_a, image_b, width_b, height_b,
    );
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray_image(w: usize, h: usize, f: impl Fn(usize, usize) -> f64) -> Vec<u8> {
        let mut v = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let g = f(x, y).clamp(0.0, 255.0) as u8;
                let i = (y * w + x) * 4;
                v[i] = g;
                v[i + 1] = g;
                v[i + 2] = g;
                v[i + 3] = 255;
            }
        }
        v
    }

    /// 几组不通约频率的正弦叠加：在图像范围内既不周期、也有足够的低频结构。
    fn pattern(x: usize, y: usize) -> f64 {
        let (x, y) = (x as f64, y as f64);
        let mut v = 0.0;
        for (fx, fy, ph) in [
            (0.0131, 0.0179, 0.0),
            (0.0413, -0.0267, 1.3),
            (0.0089, 0.0531, 2.1),
            (0.1071, 0.0917, 0.7),
            (0.1733, -0.1371, 2.7),
        ] {
            v += (x * fx + y * fy + ph).sin();
        }
        128.0 + v * 45.0
    }

    #[test]
    fn tukey_edges_taper_to_zero() {
        let w = tukey(11, 0.5);
        assert!(w[0] < 1e-9);
        assert!(w[10] < 1e-9);
        assert!((w[5] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn offset_of_identical_frames() {
        let a = gray_image(96, 96, pattern);
        let b = a.clone();
        let r = match_offset(&a, 96, 96, &b, 96, 96);
        assert!(r.dx.abs() <= 1.0, "dx={}", r.dx);
        assert!(r.dy.abs() <= 1.0, "dy={}", r.dy);
        assert!(r.confidence > 0.9, "ncc={}", r.confidence);
        assert!((r.overlap - 1.0).abs() < 1e-9);
    }

    #[test]
    fn detects_translation() {
        let (w, h) = (128, 128);
        let a = gray_image(w, h, pattern);
        let b = gray_image(w, h, |x, y| pattern(x + 11, y + 7));
        let r = match_offset(&a, w, h, &b, w, h);
        assert!((r.dx + 11.0).abs() <= 1.5, "dx={}", r.dx);
        assert!((r.dy + 7.0).abs() <= 1.5, "dy={}", r.dy);
        assert!(r.confidence > 0.8, "ncc={}", r.confidence);
    }

    #[test]
    fn detects_offset_beyond_half_padding() {
        // 大图 + 小图，真实偏移超出旧实现的 ±padded/2 表示范围
        let (aw, ah) = (400, 400);
        let (bw, bh) = (100, 100);
        let a = gray_image(aw, ah, pattern);
        let b = gray_image(bw, bh, |x, y| pattern(x + 300, y + 300));
        let r = match_offset(&a, aw, ah, &b, bw, bh);
        assert!((r.dx + 300.0).abs() <= 3.0, "dx={}", r.dx);
        assert!((r.dy + 300.0).abs() <= 3.0, "dy={}", r.dy);
        assert!(r.overlap > 0.9, "overlap={}", r.overlap);
    }

    /// 另一组不通约频率，和 pattern 没有固定相位关系
    fn pattern_b(x: usize, y: usize) -> f64 {
        let (x, y) = (x as f64, y as f64);
        let mut v = 0.0;
        for (fx, fy, ph) in [
            (0.0197, 0.0113, 0.4),
            (0.0531, 0.0371, 2.6),
            (0.0071, 0.0419, 1.1),
            (0.1317, 0.0991, 0.2),
        ] {
            v += (x * fx + y * fy + ph).sin();
        }
        128.0 + v * 50.0
    }

    #[test]
    fn unmatched_images_have_low_confidence() {
        let a = gray_image(128, 128, pattern);
        let b = gray_image(128, 128, pattern_b);
        let r = match_offset(&a, 128, 128, &b, 128, 128);
        assert!(r.confidence < 0.5, "ncc={}", r.confidence);
    }

    #[test]
    fn partial_overlap_is_matched() {
        let (w, h) = (256, 256);
        let a = gray_image(w, h, pattern);
        // B 取 A 的右下区域并向右下延伸，两者互不包含
        let b = gray_image(w, h, |x, y| pattern(x + 190, y + 190));
        let r = match_offset(&a, w, h, &b, w, h);
        assert!((r.dx + 190.0).abs() <= 2.0, "dx={}", r.dx);
        assert!((r.dy + 190.0).abs() <= 2.0, "dy={}", r.dy);
        assert!(r.confidence > 0.8, "ncc={}", r.confidence);
        assert!(r.overlap < 0.1, "overlap={}", r.overlap);
    }
}
