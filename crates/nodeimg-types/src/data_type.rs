use crate::value::Value;
use std::collections::HashMap;

/// Unique identifier for a data type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataTypeId(pub String);

impl DataTypeId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Metadata for a registered data type.
pub struct DataTypeInfo {
    pub id: DataTypeId,
    pub name: String,
}

/// Conversion function type alias.
type ConversionFn = Box<dyn Fn(Value) -> Value>;

/// Manages all data types, compatibility rules, and conversion functions.
pub struct DataTypeRegistry {
    types: HashMap<DataTypeId, DataTypeInfo>,
    conversions: HashMap<(DataTypeId, DataTypeId), ConversionFn>,
}

impl Default for DataTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DataTypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            conversions: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: DataTypeInfo) {
        self.types.insert(info.id.clone(), info);
    }

    pub fn get(&self, id: &DataTypeId) -> Option<&DataTypeInfo> {
        self.types.get(id)
    }

    pub fn register_conversion(
        &mut self,
        from: DataTypeId,
        to: DataTypeId,
        f: impl Fn(Value) -> Value + 'static,
    ) {
        self.conversions.insert((from, to), Box::new(f));
    }

    /// Same type or conversion exists.
    pub fn is_compatible(&self, from: &DataTypeId, to: &DataTypeId) -> bool {
        from == to || self.conversions.contains_key(&(from.clone(), to.clone()))
    }

    /// Convert a value. Returns None if no conversion available and types differ.
    pub fn convert(&self, value: Value, from: &DataTypeId, to: &DataTypeId) -> Option<Value> {
        if from == to {
            return Some(value);
        }
        self.conversions
            .get(&(from.clone(), to.clone()))
            .map(|f| f(value))
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();

        reg.register(DataTypeInfo {
            id: DataTypeId::new("image"),
            name: "Image".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("mask"),
            name: "Mask".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("float"),
            name: "Float".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("int"),
            name: "Int".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("color"),
            name: "Color".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("boolean"),
            name: "Boolean".into(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("string"),
            name: "String".into(),
        });

        reg.register_conversion(
            DataTypeId::new("int"),
            DataTypeId::new("float"),
            |v| match v {
                Value::Int(i) => Value::Float(i as f32),
                other => other,
            },
        );
        reg.register_conversion(
            DataTypeId::new("float"),
            DataTypeId::new("int"),
            |v| match v {
                Value::Float(f) => Value::Int(f.round() as i32),
                other => other,
            },
        );
        reg.register_conversion(
            DataTypeId::new("boolean"),
            DataTypeId::new("int"),
            |v| match v {
                Value::Boolean(b) => Value::Int(if b { 1 } else { 0 }),
                other => other,
            },
        );
        reg.register_conversion(
            DataTypeId::new("int"),
            DataTypeId::new("boolean"),
            |v| match v {
                Value::Int(i) => Value::Boolean(i != 0),
                other => other,
            },
        );
        reg.register_conversion(
            DataTypeId::new("boolean"),
            DataTypeId::new("float"),
            |v| match v {
                Value::Boolean(b) => Value::Float(if b { 1.0 } else { 0.0 }),
                other => other,
            },
        );
        reg.register_conversion(
            DataTypeId::new("float"),
            DataTypeId::new("boolean"),
            |v| match v {
                Value::Float(f) => Value::Boolean(f != 0.0),
                other => other,
            },
        );
        reg.register_conversion(DataTypeId::new("mask"), DataTypeId::new("image"), |v| {
            // TODO: convert grayscale Mask to RGBA Image (copy gray to RGB, alpha=255)
            v // pass-through placeholder
        });
        reg.register_conversion(DataTypeId::new("image"), DataTypeId::new("mask"), |v| {
            // TODO: extract luminance channel from Image to produce Mask
            v // pass-through placeholder
        });

        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_id_equality() {
        let a = DataTypeId::new("float");
        let b = DataTypeId::new("float");
        let c = DataTypeId::new("int");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_value_clone() {
        let v = Value::Float(1.0);
        let v2 = v.clone();
        match v2 {
            Value::Float(f) => assert_eq!(f, 1.0),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_register_and_query_type() {
        let mut reg = DataTypeRegistry::new();
        reg.register(DataTypeInfo {
            id: DataTypeId::new("float"),
            name: "Float".to_string(),
        });
        let info = reg.get(&DataTypeId::new("float"));
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "Float");
    }

    #[test]
    fn test_compatibility_check() {
        let mut reg = DataTypeRegistry::new();
        reg.register(DataTypeInfo {
            id: DataTypeId::new("int"),
            name: "Int".to_string(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("float"),
            name: "Float".to_string(),
        });
        reg.register_conversion(
            DataTypeId::new("int"),
            DataTypeId::new("float"),
            |v| match v {
                Value::Int(i) => Value::Float(i as f32),
                other => other,
            },
        );
        assert!(reg.is_compatible(&DataTypeId::new("int"), &DataTypeId::new("float")));
        assert!(reg.is_compatible(&DataTypeId::new("float"), &DataTypeId::new("float")));
        assert!(!reg.is_compatible(&DataTypeId::new("float"), &DataTypeId::new("int")));
    }

    #[test]
    fn test_convert_value() {
        let mut reg = DataTypeRegistry::new();
        reg.register(DataTypeInfo {
            id: DataTypeId::new("int"),
            name: "Int".to_string(),
        });
        reg.register(DataTypeInfo {
            id: DataTypeId::new("float"),
            name: "Float".to_string(),
        });
        reg.register_conversion(
            DataTypeId::new("int"),
            DataTypeId::new("float"),
            |v| match v {
                Value::Int(i) => Value::Float(i as f32),
                other => other,
            },
        );
        let result = reg.convert(
            Value::Int(42),
            &DataTypeId::new("int"),
            &DataTypeId::new("float"),
        );
        match result {
            Some(Value::Float(f)) => assert_eq!(f, 42.0),
            _ => panic!("conversion failed"),
        }
    }

    #[test]
    fn test_builtin_types_registered() {
        let reg = DataTypeRegistry::with_builtins();
        assert!(reg.get(&DataTypeId::new("image")).is_some());
        assert!(reg.get(&DataTypeId::new("mask")).is_some());
        assert!(reg.get(&DataTypeId::new("float")).is_some());
        assert!(reg.get(&DataTypeId::new("int")).is_some());
        assert!(reg.get(&DataTypeId::new("color")).is_some());
        assert!(reg.get(&DataTypeId::new("boolean")).is_some());
        assert!(reg.get(&DataTypeId::new("string")).is_some());
    }

    #[test]
    fn test_builtin_conversions() {
        let reg = DataTypeRegistry::with_builtins();
        assert!(reg.is_compatible(&DataTypeId::new("int"), &DataTypeId::new("float")));
        assert!(reg.is_compatible(&DataTypeId::new("float"), &DataTypeId::new("int")));
        assert!(reg.is_compatible(&DataTypeId::new("boolean"), &DataTypeId::new("int")));
        assert!(reg.is_compatible(&DataTypeId::new("boolean"), &DataTypeId::new("float")));
        assert!(reg.is_compatible(&DataTypeId::new("image"), &DataTypeId::new("mask")));
        assert!(reg.is_compatible(&DataTypeId::new("mask"), &DataTypeId::new("image")));
    }
}
