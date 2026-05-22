use wasm_bindgen::prelude::*;

/// Complex number representation
#[derive(Clone, Copy, Debug)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    fn scale(self, s: f64) -> Self {
        Self {
            re: self.re * s,
            im: self.im * s,
        }
    }

    fn magnitude(self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
}

/// 1D FFT using Cooley-Tukey algorithm (radix-2, in-place)
fn fft(data: &mut [Complex], inverse: bool) {
    let n = data.len();
    assert!(n.is_power_of_two(), "FFT size must be power of 2");

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }

    // Cooley-Tukey butterfly
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = if inverse {
            2.0 * std::f64::consts::PI / len as f64
        } else {
            -2.0 * std::f64::consts::PI / len as f64
        };
        let wn = Complex::new(angle.cos(), angle.sin());

        for i in (0..n).step_by(len) {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..half {
                let u = data[i + k];
                let t = w.mul(data[i + k + half]);
                data[i + k] = Complex::new(u.re + t.re, u.im + t.im);
                data[i + k + half] = Complex::new(u.re - t.re, u.im - t.im);
                w = w.mul(wn);
            }
        }
        len <<= 1;
    }

    // Scale for inverse FFT
    if inverse {
        let scale = 1.0 / n as f64;
        for v in data.iter_mut() {
            v.re *= scale;
            v.im *= scale;
        }
    }
}

/// 2D FFT using row-column decomposition
fn fft_2d(data: &mut [Complex], width: usize, height: usize, inverse: bool) {
    // FFT on each row
    for y in 0..height {
        let start = y * width;
        fft(&mut data[start..start + width], inverse);
    }

    // FFT on each column
    let mut col = vec![Complex::new(0.0, 0.0); height];
    for x in 0..width {
        for y in 0..height {
            col[y] = data[y * width + x];
        }
        fft(&mut col, inverse);
        for y in 0..height {
            data[y * width + x] = col[y];
        }
    }
}

/// Find the next power of 2 >= n
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

/// Convert RGBA pixel data to grayscale f64 array
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

/// Pad grayscale image to padded_width x padded_height (centered)
fn pad_image(
    gray: &[f64],
    width: usize,
    height: usize,
    padded_width: usize,
    padded_height: usize,
) -> Vec<Complex> {
    let mut padded = vec![Complex::new(0.0, 0.0); padded_width * padded_height];
    for y in 0..height {
        for x in 0..width {
            padded[y * padded_width + x] = Complex::new(gray[y * width + x], 0.0);
        }
    }
    padded
}

/// Apply Hanning window to reduce spectral leakage
fn apply_hanning(gray: &mut [f64], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let wx = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * x as f64 / width as f64).cos());
            let wy = 0.5
                * (1.0 - (2.0 * std::f64::consts::PI * y as f64 / height as f64).cos());
            gray[y * width + x] *= wx * wy;
        }
    }
}

/// Phase correlation to find translation offset between two images
/// Returns (dx, dy) offset: image_b = image_a shifted by (dx, dy)
fn phase_correlate(
    gray_a: &[f64],
    width_a: usize,
    height_a: usize,
    gray_b: &[f64],
    width_b: usize,
    height_b: usize,
) -> (f64, f64) {
    // Padded size (power of 2 for FFT)
    let padded_w = next_power_of_2(width_a + width_b);
    let padded_h = next_power_of_2(height_a + height_b);

    // Apply window function to reduce edge effects
    let mut wa = gray_a.to_vec();
    let mut wb = gray_b.to_vec();
    apply_hanning(&mut wa, width_a, height_a);
    apply_hanning(&mut wb, width_b, height_b);

    // Pad images
    let mut fa = pad_image(&wa, width_a, height_a, padded_w, padded_h);
    let mut fb = pad_image(&wb, width_b, height_b, padded_w, padded_h);

    // Forward FFT
    fft_2d(&mut fa, padded_w, padded_h, false);
    fft_2d(&mut fb, padded_w, padded_h, false);

    // Compute cross-power spectrum
    // CPS = F1* * F2 / |F1* * F2|
    let n = padded_w * padded_h;
    let mut cps = vec![Complex::new(0.0, 0.0); n];
    for i in 0..n {
        let f1_conj = fa[i].conj();
        let product = f1_conj.mul(fb[i]);
        let mag = product.magnitude();
        if mag > 1e-10 {
            cps[i] = product.scale(1.0 / mag);
        } else {
            cps[i] = Complex::new(0.0, 0.0);
        }
    }

    // Inverse FFT to get correlation surface
    fft_2d(&mut cps, padded_w, padded_h, true);

    // Find peak
    let mut max_val = 0.0f64;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for y in 0..padded_h {
        for x in 0..padded_w {
            let val = cps[y * padded_w + x].re;
            if val > max_val {
                max_val = val;
                max_x = x;
                max_y = y;
            }
        }
    }

    // Convert to signed offset (handle wrap-around)
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

/// Result of image matching
#[derive(serde::Serialize)]
pub struct MatchResult {
    pub dx: f64,
    pub dy: f64,
    pub confidence: f64,
}

/// Main entry point: find offset between two images
/// image_a: RGBA pixel data of image A
/// width_a, height_a: dimensions of image A
/// image_b: RGBA pixel data of image B
/// width_b, height_b: dimensions of image B
/// Returns MatchResult with dx, dy offset and confidence score
#[wasm_bindgen]
pub fn find_offset(
    image_a: &[u8],
    width_a: usize,
    height_a: usize,
    image_b: &[u8],
    width_b: usize,
    height_b: usize,
) -> JsValue {
    let gray_a = rgba_to_grayscale(image_a, width_a, height_a);
    let gray_b = rgba_to_grayscale(image_b, width_b, height_b);

    let (dx, dy) = phase_correlate(&gray_a, width_a, height_a, &gray_b, width_b, height_b);

    // Compute confidence: normalize the peak value
    // Re-run to get peak strength
    let padded_w = next_power_of_2(width_a + width_b);
    let padded_h = next_power_of_2(height_a + height_b);
    let mut wa = gray_a.clone();
    let mut wb = gray_b.clone();
    apply_hanning(&mut wa, width_a, height_a);
    apply_hanning(&mut wb, width_b, height_b);
    let mut fa = pad_image(&wa, width_a, height_a, padded_w, padded_h);
    let mut fb = pad_image(&wb, width_b, height_b, padded_w, padded_h);
    fft_2d(&mut fa, padded_w, padded_h, false);
    fft_2d(&mut fb, padded_w, padded_h, false);
    let n = padded_w * padded_h;
    let mut cps = vec![Complex::new(0.0, 0.0); n];
    for i in 0..n {
        let f1_conj = fa[i].conj();
        let product = f1_conj.mul(fb[i]);
        let mag = product.magnitude();
        if mag > 1e-10 {
            cps[i] = product.scale(1.0 / mag);
        }
    }
    fft_2d(&mut cps, padded_w, padded_h, true);

    let mut max_val = 0.0f64;
    let mut sum_sq = 0.0f64;
    for y in 0..padded_h {
        for x in 0..padded_w {
            let val = cps[y * padded_w + x].re;
            if val > max_val {
                max_val = val;
            }
            sum_sq += val * val;
        }
    }
    let rms = (sum_sq / n as f64).sqrt();
    let confidence = if rms > 1e-10 { max_val / rms } else { 0.0 };

    let result = MatchResult {
        dx,
        dy,
        confidence,
    };
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_roundtrip() {
        let n = 8;
        let mut data: Vec<Complex> = (0..n).map(|i| Complex::new(i as f64, 0.0)).collect();
        let original = data.clone();
        fft(&mut data, false);
        fft(&mut data, true);
        for i in 0..n {
            assert!((data[i].re - original[i].re).abs() < 1e-10);
            assert!((data[i].im - original[i].im).abs() < 1e-10);
        }
    }

    #[test]
    fn test_phase_correlation_identity() {
        let w = 16;
        let h = 16;
        let gray_a: Vec<f64> = (0..w * h).map(|i| (i as f64).sin()).collect();
        let gray_b = gray_a.clone();
        let (dx, dy) = phase_correlate(&gray_a, w, h, &gray_b, w, h);
        assert!((dx).abs() < 1.0, "dx should be ~0, got {}", dx);
        assert!((dy).abs() < 1.0, "dy should be ~0, got {}", dy);
    }

    #[test]
    fn test_phase_correlation_shifted() {
        let w = 32;
        let h = 32;
        let shift_x = 5;
        let shift_y = 3;

        // Create image A with some pattern
        let mut gray_a = vec![0.0f64; w * h];
        for y in 5..15 {
            for x in 10..20 {
                gray_a[y * w + x] = 255.0;
            }
        }

        // Create image B as shifted version of A
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
        // The offset should indicate B is shifted relative to A
        assert!(
            (dx - shift_x as f64).abs() < 2.0,
            "dx should be ~{}, got {}",
            shift_x,
            dx
        );
        assert!(
            (dy - shift_y as f64).abs() < 2.0,
            "dy should be ~{}, got {}",
            shift_y,
            dy
        );
    }
}
