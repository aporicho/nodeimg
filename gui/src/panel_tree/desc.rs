use crate::renderer::Color;

/// 视图描述：view() 函数返回的轻量描述树。
/// 每帧生成，用于和旧的面板树 diff。
pub enum Desc {
    Column {
        children: Vec<Desc>,
    },
    Button {
        id: &'static str,
        label: &'static str,
        color: Color,
    },
}

impl Desc {
    pub fn id(&self) -> &'static str {
        match self {
            Desc::Column { .. } => "__column",
            Desc::Button { id, .. } => id,
        }
    }

    /// 属性 hash，用于 diff 时判断是否变化。
    pub fn props_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Desc::Column { children } => {
                0u8.hash(&mut hasher);
                children.len().hash(&mut hasher);
            }
            Desc::Button { id, label, color } => {
                1u8.hash(&mut hasher);
                id.hash(&mut hasher);
                label.hash(&mut hasher);
                color.r.to_bits().hash(&mut hasher);
                color.g.to_bits().hash(&mut hasher);
                color.b.to_bits().hash(&mut hasher);
                color.a.to_bits().hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}
