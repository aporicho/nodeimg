#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ease {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    #[default]
    OutCubic,
}

impl Ease {
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::InQuad => t * t,
            Self::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::InOutQuad if t < 0.5 => 2.0 * t * t,
            Self::InOutQuad => 1.0 - (-2.0 * t + 2.0).powi(2) * 0.5,
            Self::OutCubic => 1.0 - (1.0 - t).powi(3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Ease;

    #[test]
    fn easing_samples_are_clamped() {
        assert_eq!(Ease::Linear.sample(-1.0), 0.0);
        assert_eq!(Ease::Linear.sample(2.0), 1.0);
    }

    #[test]
    fn out_cubic_reaches_endpoints() {
        assert_eq!(Ease::OutCubic.sample(0.0), 0.0);
        assert_eq!(Ease::OutCubic.sample(1.0), 1.0);
    }
}
