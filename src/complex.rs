use std::ops::{Add, Sub, Mul, MulAssign, Div, DivAssign};

// TODO: Make Complex parametric instead of f64

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Complex {
    pub real : f64,
    pub imaginary : f64,
}

impl Complex {
    pub fn real(real: f64) -> Self {
        Complex {
            real,
            imaginary: 0.,
        }
    }

    pub fn complex(real: f64, imaginary: f64) -> Self {
        Complex {
            real,
            imaginary,
        }
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Complex {
            real: self.real + rhs.real,
            imaginary: self.imaginary + rhs.imaginary
        }
    }
}

impl Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Complex {
            real: self.real - rhs.real,
            imaginary: self.imaginary - rhs.imaginary
        }
    }
}

impl Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        let divisor = (rhs.real.powf(2.)) + (rhs.imaginary.powf(2.));
        Complex {
            real: ((self.real * rhs.real) + (self.imaginary * rhs.imaginary)) / divisor,
            imaginary: ((self.imaginary * rhs.real) - (self.real * rhs.imaginary)) / divisor,
        }
    }
}

impl DivAssign for Complex {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Complex {
            real: (self.real * rhs.real) - (self.imaginary * rhs.imaginary),
            imaginary: (self.imaginary * rhs.real) + (self.real * rhs.imaginary)
        }
    }
}

impl MulAssign for Complex {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

#[cfg(test)]
mod complex_test {
    use super::Complex;

    #[test]
    fn complex_add() {
        let a = Complex::complex(7., 2.);
        let b = Complex::complex(3., 4.);
        assert_eq!(a + b, Complex::complex(10., 6.));
    }

    #[test]
    fn complex_sub() {
        let a = Complex::complex(7., 2.);
        let b = Complex::complex(3., 4.);
        assert_eq!(a - b, Complex::complex(4., -2.));
    }

    #[test]
    fn complex_mul() {
        let a = Complex::complex(7., 2.);
        let b = Complex::complex(3., 4.);
        assert_eq!(a * b, Complex::complex(13., 34.));
    }

    #[test]
    fn complex_div() {
        assert_eq!(Complex::complex(4., 2.) / Complex::complex(0., 3.), Complex::complex(2. / 3., - 4. / 3.));

    }
}
