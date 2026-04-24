pub trait Lerp: Copy {
    fn lerp(self, to: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, to: Self, t: f32) -> Self {
        self + (to - self) * t.clamp(0.0, 1.0)
    }
}

impl Lerp for [f32; 2] {
    fn lerp(self, to: Self, t: f32) -> Self {
        [self[0].lerp(to[0], t), self[1].lerp(to[1], t)]
    }
}

#[cfg(test)]
mod tests {
    use super::Lerp;

    #[test]
    fn f32_lerp_interpolates() {
        assert_eq!(0.0f32.lerp(10.0, 0.25), 2.5);
    }

    #[test]
    fn pair_lerp_interpolates() {
        assert_eq!([0.0, 10.0].lerp([10.0, 20.0], 0.5), [5.0, 15.0]);
    }
}
