use types::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cacheability {
    Cache,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachePolicy {
    pub cacheability: Cacheability,
}

impl CachePolicy {
    pub const fn cache() -> Self {
        Self {
            cacheability: Cacheability::Cache,
        }
    }

    pub const fn skip() -> Self {
        Self {
            cacheability: Cacheability::Skip,
        }
    }

    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Image(_)
            | Value::Handle(_)
            | Value::Float(_)
            | Value::Int(_)
            | Value::Bool(_)
            | Value::Color(_)
            | Value::String(_) => Self::cache(),
        }
    }

    pub fn should_cache(self) -> bool {
        matches!(self.cacheability, Cacheability::Cache)
    }
}

#[cfg(test)]
mod tests {
    use types::{DataType, Handle, Value};

    use super::{CachePolicy, Cacheability};

    #[test]
    fn value_policy_defaults_to_cache() {
        assert_eq!(
            CachePolicy::from_value(&Value::Int(1)).cacheability,
            Cacheability::Cache
        );
        assert_eq!(
            CachePolicy::from_value(&Value::Handle(Handle::new(
                "h1",
                DataType::handle(),
                "python",
                1
            )))
            .cacheability,
            Cacheability::Cache
        );
    }

    #[test]
    fn explicit_skip_policy_disables_cache() {
        assert!(!CachePolicy::skip().should_cache());
    }
}
