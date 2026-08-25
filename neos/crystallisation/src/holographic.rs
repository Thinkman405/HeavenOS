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
    /// Forward DFT.
    pub fn transform(grid: &PixelGrid) -> Self {
        let (h, w) = (grid.height(), grid.width());
        let mut coeffs = vec![Complex::default(); h * w];
        for u in 0..h {
            for v in 0..w {
                let (mut re, mut im) = (0.0, 0.0);
                for y in 0..h {
                    for x in 0..w {
                        let ang = -std::f64::consts::TAU
                            * (u as f64 * y as f64 / h as f64 + v as f64 * x as f64 / w as f64);
                        let p = grid.at(y, x);
                        re += p * ang.cos();
                        im += p * ang.sin();
                    }
                }
                coeffs[u * w + v] = Complex::new(re, im);
            }
        }
        Self {
            height: h,
            width: w,
            coeffs,
        }
    }

    /// Inverse DFT. Must recover the original grid.
    pub fn inverse(&self) -> PixelGrid {
        let (h, w) = (self.height, self.width);
        let mut pixels = vec![0.0; h * w];
        for y in 0..h {
            for x in 0..w {
                let mut re = 0.0;
                for u in 0..h {
                    for v in 0..w {
                        let ang = std::f64::consts::TAU
                            * (u as f64 * y as f64 / h as f64 + v as f64 * x as f64 / w as f64);
                        let c = self.coeffs[u * w + v];
                        re += c.re * ang.cos() - c.im * ang.sin();
                    }
                }
                pixels[y * w + x] = re / (h * w) as f64;
            }
        }
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
