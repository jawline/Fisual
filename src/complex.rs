/**
 *  We use this complex number implementation to implement
 *  fast-fourier transforms using an algorithm described
 *  at https://cp-algorithms.com/algebra/fft.html
 */
use num::traits::Float;
use std::ops::{Add, Div, DivAssign, Mul, MulAssign, Sub};

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub struct Complex<T: Float> {
    pub real: T,
    pub imaginary: T,
}

impl<T: Float> Complex<T> {
    pub fn real(real: T) -> Self {
        Complex {
            real,
            imaginary: T::zero(),
        }
    }

    pub fn complex(real: T, imaginary: T) -> Self {
        Complex { real, imaginary }
    }
}

impl<T: Float> Add for Complex<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Complex {
            real: self.real + rhs.real,
            imaginary: self.imaginary + rhs.imaginary,
        }
    }
}

impl<T: Float> Sub for Complex<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Complex {
            real: self.real - rhs.real,
            imaginary: self.imaginary - rhs.imaginary,
        }
    }
}

impl<T: Float> Div for Complex<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        let divisor = (rhs.real.powi(2)) + (rhs.imaginary.powi(2));
        Complex {
            real: ((self.real * rhs.real) + (self.imaginary * rhs.imaginary)) / divisor,
            imaginary: ((self.imaginary * rhs.real) - (self.real * rhs.imaginary)) / divisor,
        }
    }
}

impl<T: Float> DivAssign for Complex<T> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<T: Float> Mul for Complex<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Complex {
            real: (self.real * rhs.real) - (self.imaginary * rhs.imaginary),
            imaginary: (self.imaginary * rhs.real) + (self.real * rhs.imaginary),
        }
    }
}

impl<T: Float> MulAssign for Complex<T> {
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
        assert_eq!(
            Complex::complex(4., 2.) / Complex::complex(0., 3.),
            Complex::complex(2. / 3., -4. / 3.)
        );
    }
}
