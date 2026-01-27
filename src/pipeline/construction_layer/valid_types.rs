use crate::pipeline::construction_layer::pipeline_traits::HasDefault;
use num::Complex;

pub trait ValidFloat {}
impl ValidFloat for f32 {}
impl ValidFloat for f64 {}

pub trait ValidComplex {}
impl<T: ValidFloat> ValidComplex for Complex<T> {}

pub trait ValidBytes {}
impl ValidBytes for u8 {}

pub trait ValidDSPNumerical {}
impl<T: ValidFloat> ValidDSPNumerical for Complex<T> {}
impl ValidDSPNumerical for f32 {}
impl ValidDSPNumerical for f64 {}

impl HasDefault for u8 {
    fn default() -> Self {
        0
    }
}
impl HasDefault for u32 {
    fn default() -> Self {
        0
    }
}
impl HasDefault for Complex<f32> {
    fn default() -> Self {
        Complex::new(0.0, 0.0)
    }
}
impl HasDefault for Complex<f64> {
    fn default() -> Self {
        Complex::new(0.0, 0.0)
    }
}
impl HasDefault for bool {
    fn default() -> Self {
        false
    }
}
impl<T: HasDefault> HasDefault for Vec<T> {
    fn default() -> Self {
        Vec::new()
    }
}

impl HasDefault for () {
    fn default() -> Self {}
}

impl HasDefault for f32 {
    fn default() -> Self {
        0.0
    }
}
impl HasDefault for f64 {
    fn default() -> Self {
        0.0
    }
}
