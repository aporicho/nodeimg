use serde_json;

/// 参数约束，开放类型。type_id 标识约束种类，params 存储具体参数。
#[derive(Clone, Debug)]
pub struct Constraint {
    pub type_id: String,
    pub params: serde_json::Value,
}

impl Constraint {
    /// range 约束：数值范围
    pub fn range(min: f64, max: f64) -> Self {
        Self {
            type_id: "range".into(),
            params: serde_json::json!({ "min": min, "max": max }),
        }
    }

    /// enum 约束：枚举选项
    pub fn enum_options(options: Vec<String>) -> Self {
        Self {
            type_id: "enum".into(),
            params: serde_json::json!({ "options": options }),
        }
    }

    /// file_path 约束：文件扩展名过滤
    pub fn file_path(extensions: Vec<String>) -> Self {
        Self {
            type_id: "file_path".into(),
            params: serde_json::json!({ "extensions": extensions }),
        }
    }
}
