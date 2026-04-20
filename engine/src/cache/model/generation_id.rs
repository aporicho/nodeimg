#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationId(pub u64);

impl GenerationId {
    pub fn initial() -> Self {
        Self(0)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::GenerationId;

    #[test]
    fn generation_advances_monotonically() {
        let first = GenerationId::initial();
        let second = first.next();

        assert!(second > first);
        assert_eq!(second.0, 1);
    }
}
