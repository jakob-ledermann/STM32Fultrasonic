use micromath::F32Ext;

pub trait ComplexExt<T> {
    /// Calculate |self|
    fn norm(self) -> T;

    /// Calculate the principal Arg of self.
    fn arg(self) -> T;

    /// Convert to polar form (r, theta), such that
    /// `self = r * exp(i * theta)`
    fn to_polar(self) -> (T, T);

    fn exp(self) -> Self;

    fn from_polar(r: T, theta: T) -> Self;
}

impl ComplexExt<f32> for ::num_complex::Complex<f32> {
    /// Calculate |self|
    #[inline]
    fn norm(self) -> f32 {
        self.re.hypot(self.im)
    }
    /// Calculate the principal Arg of self.
    #[inline]
    fn arg(self) -> f32 {
        self.im.atan2(self.re)
    }
    /// Convert to polar form (r, theta), such that
    /// `self = r * exp(i * theta)`
    #[inline]
    fn to_polar(self) -> (f32, f32) {
        (self.norm(), self.arg())
    }

    #[inline]
    fn exp(self) -> Self {
        Self::from_polar(self.re.exp(), self.im)
    }

    #[inline]
    fn from_polar(r: f32, theta: f32) -> Self {
        Self::new(r * theta.cos(), r * theta.sin())
    }
}
