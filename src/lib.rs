use rustfft::{num_complex::Complex, FftPlanner};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static PLANNER: RefCell<FftPlanner<f64>> = RefCell::new(FftPlanner::new());
}

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
    let f2 = factor * factor;
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
            out[y * nw + x] = sum / f2 as f64;
        }
    }
    (out, nw, nh)
}

fn apply_hanning(gray: &mut [f64], width: usize, height: usize) {
    let mut wx_cache = vec![0.0f64; width];
    let mut wy_cache = vec![0.0f64; height];
    for x in 0..width {
        wx_cache[x] = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * x as f64 / width as f64).cos());
    }
    for y in 0..height {
        wy_cache[y] = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * y as f64 / height as f64).cos());
    }
    for y in 0..height {
        let wy = wy_cache[y];
        for x in 0..width {
            gray[y * width + x] *= wx_cache[x] * wy;
        }
    }
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

fn phase_correlate(
    gray_a: &[f64],
    wa: usize,
    ha: usize,
    gray_b: &[f64],
    wb: usize,
    hb: usize,
) -> (f64, f64) {
    let padded_w = next_power_of_2(wa + wb);
    let padded_h = next_power_of_2(ha + hb);

    let mut a_win = gray_a.to_vec();
    let mut b_win = gray_b.to_vec();
    apply_hanning(&mut a_win, wa, ha);
    apply_hanning(&mut b_win, wb, hb);

    let mut fa = vec![Complex::new(0.0, 0.0); padded_w * padded_h];
    let mut fb = vec![Complex::new(0.0, 0.0); padded_w * padded_h];

    for y in 0..ha {
        for x in 0..wa {
            fa[y * padded_w + x] = Complex::new(a_win[y * wa + x], 0.0);
        }
    }
    for y in 0..hb {
        for x in 0..wb {
            fb[y * padded_w + x] = Complex::new(b_win[y * wb + x], 0.0);
        }
    }

    fft_2d(&mut fa, padded_w, padded_h, false);
    fft_2d(&mut fb, padded_w, padded_h, false);

    let n = padded_w * padded_h;
    for i in 0..n {
        let conj = Complex::new(fa[i].re, -fa[i].im);
        let product = conj * fb[i];
        let mag = ((product.re * product.re + product.im * product.im) as f64).sqrt();
        if mag > 1e-10 {
            fa[i] = Complex::new(product.re / mag, product.im / mag);
        } else {
            fa[i] = Complex::new(0.0, 0.0);
        }
    }

    fft_2d(&mut fa, padded_w, padded_h, true);

    let mut max_val = 0.0f64;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for y in 0..padded_h {
        for x in 0..padded_w {
            let val = fa[y * padded_w + x].re;
            if val > max_val {
                max_val = val;
                max_x = x;
                max_y = y;
            }
        }
    }

    let dx = if max_x > padded_w / 2 {
        max_x as f64 - padded_w as f64
    } else {
        max_x as f64
    };
    let dy = if max_y > padded_h / 2 {
        max_y as f64 - padded_h as f64
    } else {
        max_y as f64
    };

    (dx, dy)
}

fn compute_confidence(
    gray_a: &[f64],
    wa: usize,
    ha: usize,
    gray_b: &[f64],
    wb: usize,
    hb: usize,
) -> f64 {
    let padded_w = next_power_of_2(wa + wb);
    let padded_h = next_power_of_2(ha + hb);

    let mut a_win = gray_a.to_vec();
    let mut b_win = gray_b.to_vec();
    apply_hanning(&mut a_win, wa, ha);
    apply_hanning(&mut b_win, wb, hb);

    let mut fa = vec![Complex::new(0.0, 0.0); padded_w * padded_h];
    let mut fb = vec![Complex::new(0.0, 0.0); padded_w * padded_h];

    for y in 0..ha {
        for x in 0..wa {
            fa[y * padded_w + x] = Complex::new(a_win[y * wa + x], 0.0);
        }
    }
    for y in 0..hb {
        for x in 0..wb {
            fb[y * padded_w + x] = Complex::new(b_win[y * wb + x], 0.0);
        }
    }

    fft_2d(&mut fa, padded_w, padded_h, false);
    fft_2d(&mut fb, padded_w, padded_h, false);

    let n = padded_w * padded_h;
    let mut max_val = 0.0f64;
    let mut sum_sq = 0.0f64;
    for i in 0..n {
        let conj = Complex::new(fa[i].re, -fa[i].im);
        let product = conj * fb[i];
        let mag = ((product.re * product.re + product.im * product.im) as f64).sqrt();
        if mag > 1e-10 {
            let val = product.re / mag;
            if val > max_val {
                max_val = val;
            }
            sum_sq += val * val;
        }
    }

    let rms = (sum_sq / n as f64).sqrt();
    if rms > 1e-10 {
        max_val / rms
    } else {
        0.0
    }
}

#[derive(serde::Serialize)]
pub struct MatchResult {
    pub dx: f64,
    pub dy: f64,
    pub confidence: f64,
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
    let gray_a_full = rgba_to_grayscale(image_a, width_a, height_a);
    let gray_b_full = rgba_to_grayscale(image_b, width_b, height_b);

    let max_dim = width_a.max(height_a).max(width_b).max(height_b);
    let factor = if max_dim > 2048 {
        8
    } else if max_dim > 1024 {
        4
    } else if max_dim > 512 {
        2
    } else {
        1
    };

    let (gray_a, wa, ha) = downsample(&gray_a_full, width_a, height_a, factor);
    let (gray_b, wb, hb) = downsample(&gray_b_full, width_b, height_b, factor);

    let (dx, dy) = phase_correlate(&gray_a, wa, ha, &gray_b, wb, hb);

    let dx_full = dx * factor as f64;
    let dy_full = dy * factor as f64;

    let confidence = compute_confidence(&gray_a, wa, ha, &gray_b, wb, hb);

    let result = MatchResult {
        dx: dx_full,
        dy: dy_full,
        confidence,
    };
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_correlation_identity() {
        let w = 16;
        let h = 16;
        let gray_a: Vec<f64> = (0..w * h).map(|i| (i as f64).sin()).collect();
        let gray_b = gray_a.clone();
        let (dx, dy) = phase_correlate(&gray_a, w, h, &gray_b, w, h);
        assert!((dx).abs() < 1.0);
        assert!((dy).abs() < 1.0);
    }

    #[test]
    fn test_phase_correlation_shifted() {
        let w = 32;
        let h = 32;
        let shift_x = 5;
        let shift_y = 3;

        let mut gray_a = vec![0.0f64; w * h];
        for y in 5..15 {
            for x in 10..20 {
                gray_a[y * w + x] = 255.0;
            }
        }

        let mut gray_b = vec![0.0f64; w * h];
        for y in 5..15 {
            for x in 10..20 {
                let nx = x + shift_x;
                let ny = y + shift_y;
                if nx < w && ny < h {
                    gray_b[ny * w + nx] = 255.0;
                }
            }
        }

        let (dx, dy) = phase_correlate(&gray_a, w, h, &gray_b, w, h);
        assert!((dx - shift_x as f64).abs() < 2.0);
        assert!((dy - shift_y as f64).abs() < 2.0);
    }

    #[test]
    fn test_downsample() {
        let gray = vec![1.0f64; 100 * 100];
        let (out, w, h) = downsample(&gray, 100, 100, 4);
        assert_eq!(w, 25);
        assert_eq!(h, 25);
        assert!((out[0] - 1.0).abs() < 1e-10);
    }
}
