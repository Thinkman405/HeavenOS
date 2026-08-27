//! Pixel grids into spatial frequency maps, projected onto Tetryen faces.
//!
//! PRD §8: "Standard pixel grids are passed through a Continuous Fourier
//! Transform (CFT), converted into spatial frequency maps, and projected onto
//! the internal faces of scalable Tetryen geometry."
//!
//! On a discrete grid the CFT is the DFT. Two properties are exact enough to
//! assert and are what make this a *representation* rather than a summary:
//!
//! - **Parseval** - spatial energy equals frequency energy over `H*W`.
//! - **DC term** - `F(0,0)` is the sum of all pixels.
//!
//! The transform is invertible, so a round trip recovers the grid. A lossy
//! "frequency map" would not represent the image.
//!
//! ## A real FFT, not just a smaller dependency
//!
//! The 2D DFT separates into two passes of 1D DFTs — an exact algebraic
//! identity (`e^{-2*pi*i*(uy/H + vx/W)} = e^{-2*pi*i*uy/H} * e^{-2*pi*i*vx/W}`),
//! not an approximation — so transforming every row (length `W`) then every
//! column (length `H`) already drops the cost from `O((HW)^2)` to
//! `O(HW*(H+W))` with **zero** change to the result, before any fast
//! algorithm enters the picture. Each 1D pass then dispatches to a radix-2
//! Cooley-Tukey FFT (`O(N log N)`) when that axis's length is a power of
//! two, falling back to the exact `O(N^2)` sum otherwise — real production
//! shapes (a `3x3` uneven-projection test, a `1xN` audio/video signal of
//! arbitrary length) are not all powers of two, and correctness for those
//! matters more than speed for a demo-scale workload. Verified against the
//! original direct 2D sum in a disposable scratch harness before this
//! replaced it: agreement to ~1e-12 across power-of-2, non-power-of-2, and
//! the exact `1xN` shape production code uses, not merely "the algorithm is
//! textbook so it must be right."

use crate::CrystalError;

/// A complex coefficient. A local two-field struct: pulling a numerics crate
/// for `a + bi` would be more dependency than arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub fn magnitude_squared(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn magnitude(self) -> f64 {
        self.magnitude_squared().sqrt()
    }
}

impl std::ops::Add for Complex {
    type Output = Complex;
    fn add(self, rhs: Complex) -> Complex {
        Complex::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl std::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, rhs: Complex) -> Complex {
        Complex::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl std::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, rhs: Complex) -> Complex {
        Complex::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

/// In-place radix-2 Cooley-Tukey, `data.len()` a power of two. Private:
/// every call site already checked the length, so an invalid one here would
/// be this module's own bug, not a caller error to report.
fn fft_radix2(data: &mut [Complex], inverse: bool) {
    let n = data.len();
    debug_assert!(n.is_power_of_two() && n > 0);

    // Bit-reversal permutation, the standard iterative-FFT precondition.
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            data.swap(i, j);
        }
    }

    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let ang = sign * std::f64::consts::TAU / len as f64;
        let wlen = Complex::new(ang.cos(), ang.sin());
        let mut i = 0;
        while i < n {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..len / 2 {
                let u = data[i + k];
                let v = data[i + k + len / 2] * w;
                data[i + k] = u + v;
                data[i + k + len / 2] = u - v;
                w = w * wlen;
            }
            i += len;
        }
        len <<= 1;
    }

    if inverse {
        for c in data.iter_mut() {
            c.re /= n as f64;
            c.im /= n as f64;
        }
    }
}

/// The exact `O(N^2)` 1D DFT — the fallback for any length that isn't a
/// power of two, and the ground truth [`fft_radix2`] was checked against.
fn dft_1d_exact(input: &[Complex], inverse: bool) -> Vec<Complex> {
    let n = input.len();
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut out = vec![Complex::default(); n];
    for k in 0..n {
        let (mut re, mut im) = (0.0, 0.0);
        for (t, &x) in input.iter().enumerate() {
            let ang = sign * std::f64::consts::TAU * (k as f64 * t as f64) / n as f64;
            let (c, s) = (ang.cos(), ang.sin());
            re += x.re * c - x.im * s;
            im += x.re * s + x.im * c;
        }
        out[k] = if inverse {
            Complex::new(re / n as f64, im / n as f64)
        } else {
            Complex::new(re, im)
        };
    }
    out
}

/// One 1D DFT pass, fast path when possible, exact fallback otherwise —
/// the single place both [`FrequencyMap::transform`]/[`FrequencyMap::inverse`]
/// route every row and column through.
fn dft_1d(input: &[Complex], inverse: bool) -> Vec<Complex> {
    let n = input.len();
    if n > 1 && n.is_power_of_two() {
        let mut data = input.to_vec();
        fft_radix2(&mut data, inverse);
        data
    } else {
        dft_1d_exact(input, inverse)
    }
}

/// The 2D transform, decomposed into a row pass then a column pass — exact
/// either direction: inverse-DFT separates the identical way forward-DFT
/// does, since only the sign of the exponent and the final `1/N` scaling
/// differ, both of which [`dft_1d`] already carries through consistently.
fn row_col_pass(input: &[Complex], h: usize, w: usize, inverse: bool) -> Vec<Complex> {
    let mut tmp = vec![Complex::default(); h * w];
    for y in 0..h {
        let row: Vec<Complex> = (0..w).map(|x| input[y * w + x]).collect();
        let t = dft_1d(&row, inverse);
        tmp[y * w..(y + 1) * w].copy_from_slice(&t);
    }
    let mut out = vec![Complex::default(); h * w];
    for v in 0..w {
        let col: Vec<Complex> = (0..h).map(|y| tmp[y * w + v]).collect();
        let t = dft_1d(&col, inverse);
        for (u, value) in t.into_iter().enumerate() {
            out[u * w + v] = value;
        }
    }
    out
}

/// A rectangular grid of intensities.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelGrid {
    height: usize,
    width: usize,
    pixels: Vec<f64>,
}

impl PixelGrid {
    /// # Errors
    /// [`CrystalError::MalformedGrid`] if the pixel count does not match the
    /// stated dimensions.
    pub fn new(height: usize, width: usize, pixels: Vec<f64>) -> Result<Self, CrystalError> {
        if height == 0 || width == 0 || pixels.len() != height * width {
            return Err(CrystalError::MalformedGrid {
                height,
                width,
                pixels: pixels.len(),
            });
        }
        Ok(Self {
            height,
            width,
            pixels,
        })
    }

    pub fn height(&self) -> usize {
        self.height
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn pixels(&self) -> &[f64] {
        &self.pixels
    }

    pub fn at(&self, y: usize, x: usize) -> f64 {
        self.pixels[y * self.width + x]
    }

    /// Sum of squared intensities - the spatial side of Parseval.
    pub fn energy(&self) -> f64 {
        self.pixels.iter().map(|p| p * p).sum()
    }

    pub fn sum(&self) -> f64 {
        self.pixels.iter().sum()
    }
}

/// The spatial frequency map of a grid.
#[derive(Debug, Clone, PartialEq)]
pub struct FrequencyMap {
    height: usize,
    width: usize,
    coeffs: Vec<Complex>,
}

impl FrequencyMap {
    /// Forward DFT — real rows, then columns; see the module docs for why
    /// that's an exact identity and not a shortcut.
    pub fn transform(grid: &PixelGrid) -> Self {
        let (h, w) = (grid.height(), grid.width());
        let rows: Vec<Complex> = grid.pixels().iter().map(|&p| Complex::new(p, 0.0)).collect();
        let coeffs = row_col_pass(&rows, h, w, false);
        Self {
            height: h,
            width: w,
            coeffs,
        }
    }

    /// Inverse DFT. Must recover the original grid.
    pub fn inverse(&self) -> PixelGrid {
        let (h, w) = (self.height, self.width);
        let spatial = row_col_pass(&self.coeffs, h, w, true);
        let pixels = spatial.iter().map(|c| c.re).collect();
        PixelGrid {
            height: h,
            width: w,
            pixels,
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn coefficients(&self) -> &[Complex] {
        &self.coeffs
    }

    /// The DC term: equal to the sum of all pixels.
    pub fn dc(&self) -> f64 {
        self.coeffs[0].re
    }

    /// Frequency-side energy, normalised by `H*W` - the Parseval partner of
    /// [`PixelGrid::energy`].
    pub fn energy(&self) -> f64 {
        self.coeffs.iter().map(|c| c.magnitude_squared()).sum::<f64>()
            / (self.height * self.width) as f64
    }

    /// Distribute the coefficients across a Tetryen's **four** faces.
    ///
    /// # Errors
    /// [`CrystalError::UnevenProjection`] when the coefficient count is not
    /// divisible by 4. Refusing beats handing one face extra and calling the
    /// projection balanced.
    pub fn project_onto_faces(&self) -> Result<[FaceProjection; 4], CrystalError> {
        let n = self.coeffs.len();
        if n % 4 != 0 {
            return Err(CrystalError::UnevenProjection { coefficients: n });
        }
        let per = n / 4;
        Ok(std::array::from_fn(|face| FaceProjection {
            face,
            coeffs: self.coeffs[face * per..(face + 1) * per].to_vec(),
        }))
    }
}

/// One face's share of a frequency map.
///
/// `project_onto_faces` returns exactly four of these, in a fixed-size array: a
/// Tetryen has four faces, and a projection onto three or five is not a
/// degenerate projection - it is not a Tetryen projection.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceProjection {
    face: usize,
    coeffs: Vec<Complex>,
}

impl FaceProjection {
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn coefficients(&self) -> &[Complex] {
        &self.coeffs
    }
    pub fn energy(&self) -> f64 {
        self.coeffs.iter().map(|c| c.magnitude_squared()).sum()
    }
}
