/// 数据类型标识符，开放类型。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataType(pub String);

impl DataType {
    pub fn image() -> Self { Self("image".into()) }
    pub fn float() -> Self { Self("float".into()) }
    pub fn int() -> Self { Self("int".into()) }
    pub fn bool() -> Self { Self("bool".into()) }
    pub fn color() -> Self { Self("color".into()) }
    pub fn string() -> Self { Self("string".into()) }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_equality() {
        assert_eq!(DataType::image(), DataType::image());
        assert_ne!(DataType::image(), DataType::float());
    }

    #[test]
    fn test_data_type_custom() {
        let custom = DataType("my_plugin_type".into());
        assert_ne!(custom, DataType::image());
    }
}
