# Node System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the monolithic `nodes.rs` with a modular, registry-based node system as specified in `docs/domain.md`, `docs/architecture.md`, and `docs/catalog.md`.

**Architecture:** Build new system alongside existing code, then swap over. Each component lives in `src/node/` with strict dependency ordering. Registries hold all type definitions; nothing is hardcoded. NodeInstance is a lightweight struct stored in `Snarl<NodeInstance>` — all metadata resolved at runtime via NodeRegistry.

**Tech Stack:** Rust, eframe/egui 0.33, egui-snarl 0.9, image 0.25, rfd 0.17, serde/serde_json

**Specs:** `docs/domain.md`, `docs/architecture.md`, `docs/catalog.md`

**Strategy:** Build new system in `src/node/`, verify each component with unit tests, wire into `app.rs`, then delete old `src/nodes.rs`.

**Scope:** V1 实现 4 个核心内置节点（LoadImage、ColorAdjust、Preview、SaveImage）。其余 23 个节点（Curves 标记为 v2 除外）在本计划完成后，按 catalog.md 逐个添加——每个节点只需在 `builtins/` 下创建文件并在 `mod.rs` 中注册，模式与 Task 20-22 完全相同。

**V1 不实现的功能（deferred to V2）：**
- **参数引脚（Parameter Pins）**：domain.md 定义"每个参数自动生成一个同类型的输入引脚"，V1 暂不实现。`inputs()` 只返回 `def.inputs.len()`（数据引脚），不包含参数引脚。**序列化兼容性注意**：V2 添加参数引脚时 `inputs()` 返回值会增大，导致引脚索引变化。因此 Serializer 按引脚**名称**（而非索引）保存连接，V2 扩展时不会破坏已有图文件。
- **Image ↔ Mask 像素级转换**：注册为兼容但转换函数为 pass-through 占位，实际像素转换待后续在 processing.rs 中实现。

---

## Chunk 1: Foundation Types + Core Registries

### Task 1: Create node module skeleton

**Files:**
- Create: `src/node/mod.rs`

- [ ] **Step 1: Create the module directory, mod.rs, and empty submodule files**

```rust
// src/node/mod.rs
pub mod types;
pub mod constraint;
pub mod category;
```

同时创建三个空文件以确保编译通过：
- `src/node/types.rs` — 内容为空
- `src/node/constraint.rs` — 内容为空
- `src/node/category.rs` — 内容为空

- [ ] **Step 2: Register the module in main.rs**

Add `mod node;` to `src/main.rs` (after existing mod declarations). If mods are declared in `app.rs`, add it there instead — check where `mod nodes;` currently lives.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles (modules are empty but exist)

---

### Task 2: Implement Value enum and DataTypeId

**Files:**
- Create: `src/node/types.rs`

- [ ] **Step 1: Write failing test for Value conversion**

```rust
// src/node/types.rs

/// Represents any value that flows through the node graph.
#[derive(Clone, Debug)]
pub enum Value {
    Image(std::sync::Arc<image::DynamicImage>),
    Mask(std::sync::Arc<image::DynamicImage>),
    Float(f32),
    Int(i32),
    Color([f32; 4]),
    Boolean(bool),
    String(String),
}

/// Unique identifier for a data type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataTypeId(pub String);

impl DataTypeId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
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
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib node::types`
Expected: 2 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/node/mod.rs src/node/types.rs
git commit -m "feat: add Value enum and DataTypeId"
```

---

### Task 3: Implement DataTypeRegistry

**Files:**
- Modify: `src/node/types.rs`

- [ ] **Step 1: Write failing test for registry**

Add to the test module in `types.rs`:

```rust
#[test]
fn test_register_and_query_type() {
    let mut reg = DataTypeRegistry::new();
    reg.register(DataTypeInfo {
        id: DataTypeId::new("float"),
        name: "Float".to_string(),
        pin_color: [1.0, 0.5, 0.0], // orange RGB
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
        pin_color: [0.0, 0.8, 0.8],
    });
    reg.register(DataTypeInfo {
        id: DataTypeId::new("float"),
        name: "Float".to_string(),
        pin_color: [1.0, 0.5, 0.0],
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
        pin_color: [0.0, 0.8, 0.8],
    });
    reg.register(DataTypeInfo {
        id: DataTypeId::new("float"),
        name: "Float".to_string(),
        pin_color: [1.0, 0.5, 0.0],
    });
    reg.register_conversion(
        DataTypeId::new("int"),
        DataTypeId::new("float"),
        |v| match v {
            Value::Int(i) => Value::Float(i as f32),
            other => other,
        },
    );
    let result = reg.convert(Value::Int(42), &DataTypeId::new("int"), &DataTypeId::new("float"));
    match result {
        Some(Value::Float(f)) => assert_eq!(f, 42.0),
        _ => panic!("conversion failed"),
    }
}
```

- [ ] **Step 2: Run tests to see them fail**

Run: `cargo test --lib node::types`
Expected: FAIL — `DataTypeRegistry` not defined

- [ ] **Step 3: Implement DataTypeRegistry**

Add above the test module in `types.rs`:

```rust
use std::collections::HashMap;

/// Metadata for a registered data type.
pub struct DataTypeInfo {
    pub id: DataTypeId,
    pub name: String,
    pub pin_color: [f32; 3], // RGB 0.0-1.0
}

/// Manages all data types, compatibility rules, and conversion functions.
pub struct DataTypeRegistry {
    types: HashMap<DataTypeId, DataTypeInfo>,
    conversions: HashMap<(DataTypeId, DataTypeId), Box<dyn Fn(Value) -> Value>>,
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

    pub fn pin_color(&self, id: &DataTypeId) -> [f32; 3] {
        self.types
            .get(id)
            .map(|t| t.pin_color)
            .unwrap_or([0.5, 0.5, 0.5])
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib node::types`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/node/types.rs
git commit -m "feat: implement DataTypeRegistry with conversion support"
```

---

### Task 4: Register built-in data types

**Files:**
- Modify: `src/node/types.rs`

- [ ] **Step 1: Write test for built-in types**

```rust
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
    // Int → Float
    assert!(reg.is_compatible(&DataTypeId::new("int"), &DataTypeId::new("float")));
    // Float → Int
    assert!(reg.is_compatible(&DataTypeId::new("float"), &DataTypeId::new("int")));
    // Boolean → Int
    assert!(reg.is_compatible(&DataTypeId::new("boolean"), &DataTypeId::new("int")));
    // Boolean → Float
    assert!(reg.is_compatible(&DataTypeId::new("boolean"), &DataTypeId::new("float")));
    // Image ↔ Mask
    assert!(reg.is_compatible(&DataTypeId::new("image"), &DataTypeId::new("mask")));
    assert!(reg.is_compatible(&DataTypeId::new("mask"), &DataTypeId::new("image")));
}
```

- [ ] **Step 2: Implement `with_builtins()`**

```rust
impl DataTypeRegistry {
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();

        // Register types
        reg.register(DataTypeInfo { id: DataTypeId::new("image"), name: "Image".into(), pin_color: [0.2, 0.4, 1.0] });
        reg.register(DataTypeInfo { id: DataTypeId::new("mask"), name: "Mask".into(), pin_color: [0.2, 0.8, 0.2] });
        reg.register(DataTypeInfo { id: DataTypeId::new("float"), name: "Float".into(), pin_color: [1.0, 0.5, 0.0] });
        reg.register(DataTypeInfo { id: DataTypeId::new("int"), name: "Int".into(), pin_color: [0.0, 0.8, 0.8] });
        reg.register(DataTypeInfo { id: DataTypeId::new("color"), name: "Color".into(), pin_color: [0.6, 0.2, 0.8] });
        reg.register(DataTypeInfo { id: DataTypeId::new("boolean"), name: "Boolean".into(), pin_color: [0.9, 0.2, 0.2] });
        reg.register(DataTypeInfo { id: DataTypeId::new("string"), name: "String".into(), pin_color: [0.5, 0.5, 0.5] });

        // Register conversions (see domain.md compatibility matrix)
        reg.register_conversion(DataTypeId::new("int"), DataTypeId::new("float"), |v| match v {
            Value::Int(i) => Value::Float(i as f32),
            other => other,
        });
        reg.register_conversion(DataTypeId::new("float"), DataTypeId::new("int"), |v| match v {
            Value::Float(f) => Value::Int(f.round() as i32),
            other => other,
        });
        reg.register_conversion(DataTypeId::new("boolean"), DataTypeId::new("int"), |v| match v {
            Value::Boolean(b) => Value::Int(if b { 1 } else { 0 }),
            other => other,
        });
        reg.register_conversion(DataTypeId::new("int"), DataTypeId::new("boolean"), |v| match v {
            Value::Int(i) => Value::Boolean(i != 0),
            other => other,
        });
        reg.register_conversion(DataTypeId::new("boolean"), DataTypeId::new("float"), |v| match v {
            Value::Boolean(b) => Value::Float(if b { 1.0 } else { 0.0 }),
            other => other,
        });
        reg.register_conversion(DataTypeId::new("float"), DataTypeId::new("boolean"), |v| match v {
            Value::Float(f) => Value::Boolean(f != 0.0),
            other => other,
        });
        // Image ↔ Mask conversions
        reg.register_conversion(DataTypeId::new("mask"), DataTypeId::new("image"), |v| {
            // TODO: convert grayscale Mask to RGBA Image (copy gray to RGB, alpha=255)
            // Requires adding a pixel-level conversion in processing.rs
            v // pass-through placeholder
        });
        reg.register_conversion(DataTypeId::new("image"), DataTypeId::new("mask"), |v| {
            // TODO: extract luminance channel from Image to produce Mask
            // Requires adding a pixel-level conversion in processing.rs
            v // pass-through placeholder
        });

        reg
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib node::types`
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/node/types.rs
git commit -m "feat: register 7 built-in data types with conversion matrix"
```

---

### Task 5: Implement ConstraintRegistry

**Files:**
- Create: `src/node/constraint.rs`

- [ ] **Step 1: Define Constraint enum and write tests**

```rust
// src/node/constraint.rs

/// Describes the valid range/options for a parameter value.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// No constraint.
    None,
    /// Numeric range (inclusive).
    Range { min: f64, max: f64 },
    /// Enumerated options: list of (label, value) pairs.
    Enum { options: Vec<(String, String)> },
    /// File path with extension filters.
    FilePath { filters: Vec<String> },
}

/// Unique identifier for a constraint type (for widget matching).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    None,
    Range,
    Enum,
    FilePath,
}

impl Constraint {
    /// Returns the constraint type identifier for widget matching.
    pub fn constraint_type(&self) -> ConstraintType {
        match self {
            Constraint::None => ConstraintType::None,
            Constraint::Range { .. } => ConstraintType::Range,
            Constraint::Enum { .. } => ConstraintType::Enum,
            Constraint::FilePath { .. } => ConstraintType::FilePath,
        }
    }

    /// Validates a float value against this constraint.
    pub fn validate_f64(&self, value: f64) -> bool {
        match self {
            Constraint::Range { min, max } => value >= *min && value <= *max,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_type() {
        let c = Constraint::Range { min: 0.0, max: 1.0 };
        assert_eq!(c.constraint_type(), ConstraintType::Range);

        let c = Constraint::Enum { options: vec![("A".into(), "a".into())] };
        assert_eq!(c.constraint_type(), ConstraintType::Enum);

        let c = Constraint::None;
        assert_eq!(c.constraint_type(), ConstraintType::None);
    }

    #[test]
    fn test_range_validation() {
        let c = Constraint::Range { min: -1.0, max: 1.0 };
        assert!(c.validate_f64(0.0));
        assert!(c.validate_f64(-1.0));
        assert!(c.validate_f64(1.0));
        assert!(!c.validate_f64(1.1));
        assert!(!c.validate_f64(-1.1));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib node::constraint`
Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/node/constraint.rs
git commit -m "feat: implement Constraint enum with validation"
```

---

### Task 6: Implement CategoryRegistry

**Files:**
- Create: `src/node/category.rs`

- [ ] **Step 1: Implement and test**

```rust
// src/node/category.rs
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CategoryId(pub String);

impl CategoryId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

pub struct CategoryInfo {
    pub id: CategoryId,
    pub name: String,
    pub weight: i32, // lower = appears first
}

pub struct CategoryRegistry {
    categories: HashMap<CategoryId, CategoryInfo>,
}

impl CategoryRegistry {
    pub fn new() -> Self {
        Self { categories: HashMap::new() }
    }

    pub fn register(&mut self, info: CategoryInfo) {
        self.categories.insert(info.id.clone(), info);
    }

    pub fn get(&self, id: &CategoryId) -> Option<&CategoryInfo> {
        self.categories.get(id)
    }

    /// Returns all categories sorted by weight.
    pub fn sorted(&self) -> Vec<&CategoryInfo> {
        let mut cats: Vec<_> = self.categories.values().collect();
        cats.sort_by_key(|c| c.weight);
        cats
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        let cats = [
            ("data", "数据型", 0),
            ("generate", "生成型", 1),
            ("color", "颜色处理型", 2),
            ("transform", "空间变换型", 3),
            ("filter", "滤镜型", 4),
            ("composite", "合成型", 5),
            ("tool", "工具型", 6),
        ];
        for (id, name, weight) in cats {
            reg.register(CategoryInfo {
                id: CategoryId::new(id),
                name: name.to_string(),
                weight,
            });
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_query() {
        let reg = CategoryRegistry::with_builtins();
        let info = reg.get(&CategoryId::new("filter"));
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "滤镜型");
    }

    #[test]
    fn test_sorted_by_weight() {
        let reg = CategoryRegistry::with_builtins();
        let sorted = reg.sorted();
        assert_eq!(sorted[0].id, CategoryId::new("data"));
        assert_eq!(sorted[6].id, CategoryId::new("tool"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib node::category`
Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/node/category.rs
git commit -m "feat: implement CategoryRegistry with 7 built-in categories"
```

---

### Task 7: Wire mod.rs exports and full test pass

**Files:**
- Modify: `src/node/mod.rs`

- [ ] **Step 1: Update mod.rs to export everything**

```rust
// src/node/mod.rs
pub mod types;
pub mod constraint;
pub mod category;

pub use types::{Value, DataTypeId, DataTypeInfo, DataTypeRegistry};
pub use constraint::{Constraint, ConstraintType};
pub use category::{CategoryId, CategoryInfo, CategoryRegistry};
```

- [ ] **Step 2: Run all node tests**

Run: `cargo test --lib node`
Expected: all tests PASS (should be ~9 tests)

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Commit**

```bash
git add src/node/mod.rs
git commit -m "feat: wire node module exports, Chunk 1 complete"
```

---

## Chunk 2: Widget System

### Task 8: Create WidgetRegistry

**Files:**
- Create: `src/node/widget/mod.rs`
- Modify: `src/node/mod.rs`

- [ ] **Step 1: Define WidgetId and WidgetRegistry**

```rust
// src/node/widget/mod.rs
pub mod slider;
pub mod checkbox;
pub mod dropdown;
pub mod file_picker;
pub mod number_input;
pub mod radio_group;
pub mod color_picker;

use crate::node::types::DataTypeId;
use crate::node::constraint::ConstraintType;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(pub String);

impl WidgetId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Key for matching data type + constraint to widget(s).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetMatchKey {
    pub data_type: DataTypeId,
    pub constraint_type: ConstraintType,
}

/// Render function signature: (ui, value, constraint, param_name, disabled) -> changed
pub type WidgetRenderFn = fn(&mut egui::Ui, &mut Value, &Constraint, &str, bool) -> bool;

pub struct WidgetEntry {
    pub id: WidgetId,
    pub is_default: bool,
    pub render: WidgetRenderFn,
}

pub struct WidgetRegistry {
    /// Maps (DataType + Constraint) → list of available widgets.
    mappings: HashMap<WidgetMatchKey, Vec<WidgetEntry>>,
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self { mappings: HashMap::new() }
    }

    pub fn register(
        &mut self,
        data_type: DataTypeId,
        constraint_type: ConstraintType,
        widget_id: WidgetId,
        is_default: bool,
        render: WidgetRenderFn,
    ) {
        let key = WidgetMatchKey { data_type, constraint_type };
        let entries = self.mappings.entry(key).or_insert_with(Vec::new);
        entries.push(WidgetEntry { id: widget_id, is_default, render });
    }

    /// Render a parameter using the specified widget (or default).
    pub fn render(
        &self,
        widget_id: Option<&WidgetId>,
        data_type: &DataTypeId,
        constraint_type: &ConstraintType,
        ui: &mut egui::Ui,
        value: &mut Value,
        constraint: &Constraint,
        param_name: &str,
        disabled: bool,
    ) -> bool {
        let key = WidgetMatchKey { data_type: data_type.clone(), constraint_type: constraint_type.clone() };
        if let Some(entries) = self.mappings.get(&key) {
            let entry = if let Some(wid) = widget_id {
                entries.iter().find(|e| &e.id == wid)
            } else {
                entries.iter().find(|e| e.is_default)
            };
            if let Some(entry) = entry {
                return (entry.render)(ui, value, constraint, param_name, disabled);
            }
        }
        false
    }

    /// Returns the default widget for a given type+constraint combo.
    pub fn default_widget(&self, data_type: &DataTypeId, constraint_type: &ConstraintType) -> Option<&WidgetId> {
        let key = WidgetMatchKey {
            data_type: data_type.clone(),
            constraint_type: constraint_type.clone(),
        };
        self.mappings.get(&key)
            .and_then(|entries| entries.iter().find(|e| e.is_default).map(|e| &e.id))
    }

    /// Returns all available widgets for a given type+constraint combo.
    pub fn available_widgets(&self, data_type: &DataTypeId, constraint_type: &ConstraintType) -> Vec<&WidgetId> {
        let key = WidgetMatchKey {
            data_type: data_type.clone(),
            constraint_type: constraint_type.clone(),
        };
        self.mappings.get(&key)
            .map(|entries| entries.iter().map(|e| &e.id).collect())
            .unwrap_or_default()
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();

        // Float + Range → Slider (default), NumberInput
        reg.register(DataTypeId::new("float"), ConstraintType::Range, WidgetId::new("slider"), true, slider::render_slider);
        reg.register(DataTypeId::new("float"), ConstraintType::Range, WidgetId::new("number_input"), false, number_input::render_number_input);

        // Int + Range → IntSlider (default), NumberInput
        reg.register(DataTypeId::new("int"), ConstraintType::Range, WidgetId::new("int_slider"), true, slider::render_int_slider);
        reg.register(DataTypeId::new("int"), ConstraintType::Range, WidgetId::new("number_input"), false, number_input::render_number_input);

        // Boolean + None → Checkbox (default)
        reg.register(DataTypeId::new("boolean"), ConstraintType::None, WidgetId::new("checkbox"), true, checkbox::render_checkbox);

        // Color + None → ColorPicker (default)
        reg.register(DataTypeId::new("color"), ConstraintType::None, WidgetId::new("color_picker"), true, color_picker::render_color_picker);

        // String + Enum → Dropdown (default), RadioGroup
        reg.register(DataTypeId::new("string"), ConstraintType::Enum, WidgetId::new("dropdown"), true, dropdown::render_dropdown);
        reg.register(DataTypeId::new("string"), ConstraintType::Enum, WidgetId::new("radio_group"), false, radio_group::render_radio_group);

        // String + FilePath → FilePicker (default)
        reg.register(DataTypeId::new("string"), ConstraintType::FilePath, WidgetId::new("file_picker"), true, file_picker::render_file_picker);

        // Float + None → NumberInput (default)
        reg.register(DataTypeId::new("float"), ConstraintType::None, WidgetId::new("number_input"), true, number_input::render_number_input);

        // Int + None → NumberInput (default)
        reg.register(DataTypeId::new("int"), ConstraintType::None, WidgetId::new("number_input"), true, number_input::render_number_input);

        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_widget_float_range() {
        let reg = WidgetRegistry::with_builtins();
        let w = reg.default_widget(&DataTypeId::new("float"), &ConstraintType::Range);
        assert_eq!(w, Some(&WidgetId::new("slider")));
    }

    #[test]
    fn test_available_widgets_float_range() {
        let reg = WidgetRegistry::with_builtins();
        let ws = reg.available_widgets(&DataTypeId::new("float"), &ConstraintType::Range);
        assert_eq!(ws.len(), 2);
    }

    #[test]
    fn test_default_widget_string_enum() {
        let reg = WidgetRegistry::with_builtins();
        let w = reg.default_widget(&DataTypeId::new("string"), &ConstraintType::Enum);
        assert_eq!(w, Some(&WidgetId::new("dropdown")));
    }
}
```

- [ ] **Step 2: Add `pub mod widget;` to `src/node/mod.rs`**

Also create empty files for each widget module so compilation works:
- `src/node/widget/slider.rs` — `// TODO`
- `src/node/widget/checkbox.rs` — `// TODO`
- `src/node/widget/dropdown.rs` — `// TODO`
- `src/node/widget/file_picker.rs` — `// TODO`
- `src/node/widget/number_input.rs` — `// TODO`
- `src/node/widget/radio_group.rs` — `// TODO`
- `src/node/widget/color_picker.rs` — `// TODO`

- [ ] **Step 3: Run tests**

Run: `cargo test --lib node::widget`
Expected: 3 tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/node/widget/
git commit -m "feat: implement WidgetRegistry with built-in mappings"
```

---

### Task 9: Implement Slider widget

**Files:**
- Modify: `src/node/widget/slider.rs`

- [ ] **Step 1: Implement slider rendering function**

```rust
// src/node/widget/slider.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

/// Renders a float slider. Returns true if value changed.
pub fn render_slider(ui: &mut egui::Ui, value: &mut Value, constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Constraint::Range { min, max } = constraint else { return false; };
    let Value::Float(ref mut v) = value else { return false; };
    ui.add_enabled(!disabled, egui::Slider::new(v, (*min as f32)..=(*max as f32))).changed()
}

/// Renders an integer slider. Returns true if value changed.
pub fn render_int_slider(ui: &mut egui::Ui, value: &mut Value, constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Constraint::Range { min, max } = constraint else { return false; };
    let Value::Int(ref mut v) = value else { return false; };
    ui.add_enabled(!disabled, egui::Slider::new(v, (*min as i32)..=(*max as i32))).changed()
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/node/widget/slider.rs
git commit -m "feat: implement slider and int_slider widgets"
```

---

### Task 10: Implement Checkbox widget

**Files:**
- Modify: `src/node/widget/checkbox.rs`

- [ ] **Step 1: Implement**

```rust
// src/node/widget/checkbox.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

/// Renders a checkbox. Returns true if value changed.
pub fn render_checkbox(ui: &mut egui::Ui, value: &mut Value, _constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Value::Boolean(ref mut v) = value else { return false; };
    ui.add_enabled(!disabled, egui::Checkbox::new(v, "")).changed()
}
```

- [ ] **Step 2: Verify compilation and commit**

Run: `cargo check`

```bash
git add src/node/widget/checkbox.rs
git commit -m "feat: implement checkbox widget"
```

---

### Task 11: Implement Dropdown, FilePicker, NumberInput, RadioGroup, ColorPicker

**Files:**
- Modify: `src/node/widget/dropdown.rs`
- Modify: `src/node/widget/file_picker.rs`
- Modify: `src/node/widget/number_input.rs`
- Modify: `src/node/widget/radio_group.rs`
- Modify: `src/node/widget/color_picker.rs`

- [ ] **Step 1: Implement Dropdown**

```rust
// src/node/widget/dropdown.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

pub fn render_dropdown(ui: &mut egui::Ui, value: &mut Value, constraint: &Constraint, param_name: &str, disabled: bool) -> bool {
    let Constraint::Enum { options } = constraint else { return false; };
    let Value::String(ref mut selected) = value else { return false; };
    let mut changed = false;
    ui.scope(|ui| {
        ui.set_enabled(!disabled);
        egui::ComboBox::from_id_salt(format!("{}-{}", ui.id().value(), param_name))
            .selected_text(
                options.iter().find(|(_, v)| v == selected).map(|(l, _)| l.as_str()).unwrap_or(selected.as_str())
            )
            .show_ui(ui, |ui| {
                for (label, val) in options {
                    if ui.selectable_label(selected == val, label).clicked() {
                        *selected = val.clone();
                        changed = true;
                    }
                }
            });
    });
    changed
}
```

- [ ] **Step 2: Implement FilePicker**

```rust
// src/node/widget/file_picker.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

pub fn render_file_picker(ui: &mut egui::Ui, value: &mut Value, constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Constraint::FilePath { filters } = constraint else { return false; };
    let Value::String(ref mut path) = value else { return false; };
    let mut changed = false;
    ui.scope(|ui| {
        ui.set_enabled(!disabled);
        ui.horizontal(|ui| {
            let display = if path.is_empty() { "Select file..." } else {
                std::path::Path::new(path.as_str())
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path.as_str())
            };
            ui.label(display);
            if ui.button("Browse").clicked() {
                let mut dialog = rfd::FileDialog::new();
                for ext in filters {
                    dialog = dialog.add_filter(ext, &[ext.as_str()]);
                }
                if let Some(p) = dialog.pick_file() {
                    *path = p.to_string_lossy().to_string();
                    changed = true;
                }
            }
        });
    });
    changed
}
```

- [ ] **Step 3: Implement NumberInput**

```rust
// src/node/widget/number_input.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

pub fn render_number_input(ui: &mut egui::Ui, value: &mut Value, _constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let changed = match value {
        Value::Float(ref mut v) => ui.add_enabled(!disabled, egui::DragValue::new(v).speed(0.01)).changed(),
        Value::Int(ref mut v) => ui.add_enabled(!disabled, egui::DragValue::new(v).speed(1.0)).changed(),
        _ => false,
    };
    changed
}
```

- [ ] **Step 4: Implement RadioGroup**

```rust
// src/node/widget/radio_group.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

pub fn render_radio_group(ui: &mut egui::Ui, value: &mut Value, constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Constraint::Enum { options } = constraint else { return false; };
    let Value::String(ref mut selected) = value else { return false; };
    let mut changed = false;
    ui.scope(|ui| {
        ui.set_enabled(!disabled);
        for (label, val) in options {
            if ui.radio_value(selected, val.clone(), label).changed() {
                changed = true;
            }
        }
    });
    changed
}
```

- [ ] **Step 5: Implement ColorPicker**

```rust
// src/node/widget/color_picker.rs
use crate::node::types::Value;
use crate::node::constraint::Constraint;

pub fn render_color_picker(ui: &mut egui::Ui, value: &mut Value, _constraint: &Constraint, _param_name: &str, disabled: bool) -> bool {
    let Value::Color(ref mut c) = value else { return false; };
    let mut changed = false;
    ui.scope(|ui| {
        ui.set_enabled(!disabled);
        changed = egui::color_picker::color_edit_button_rgba_unmultiplied(ui, c).changed();
    });
    changed
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check`
Expected: compiles

- [ ] **Step 7: Commit**

```bash
git add src/node/widget/
git commit -m "feat: implement all 7 widget types, Chunk 2 complete"
```

---

## Chunk 3: Node Registry + Data Model

### Task 12: Implement PinDef, ParamDef, NodeDef

**Files:**
- Create: `src/node/registry.rs`

- [ ] **Step 1: Define types**

```rust
// src/node/registry.rs
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId};
use crate::node::constraint::Constraint;
use crate::node::category::CategoryId;
use crate::node::widget::WidgetId;

/// Describes an input or output pin.
#[derive(Clone, Debug)]
pub struct PinDef {
    pub name: String,
    pub data_type: DataTypeId,
    pub required: bool,
}

/// Describes a node parameter.
#[derive(Clone, Debug)]
pub struct ParamDef {
    pub name: String,
    pub data_type: DataTypeId,
    pub constraint: Constraint,
    pub default: Value,
    pub widget_override: Option<WidgetId>,
}

/// Processing function signature: inputs + params → outputs.
pub type ProcessFn = Box<dyn Fn(&HashMap<String, Value>, &HashMap<String, Value>) -> HashMap<String, Value> + Send + Sync>;

/// Complete definition of a node type.
pub struct NodeDef {
    pub type_id: String,
    pub title: String,
    pub category: CategoryId,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub has_preview: bool,
    pub process: ProcessFn,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`

- [ ] **Step 3: Commit**

```bash
git add src/node/registry.rs
git commit -m "feat: define PinDef, ParamDef, NodeDef types"
```

---

### Task 13: Implement NodeInstance

**Files:**
- Modify: `src/node/registry.rs`

- [ ] **Step 1: Add NodeInstance and test**

```rust
/// Runtime node instance stored in the graph.
#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub type_id: String,
    pub params: HashMap<String, Value>,
}
```

Add test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_instance_creation() {
        let inst = NodeInstance {
            type_id: "color_adjust".into(),
            params: HashMap::from([
                ("brightness".into(), Value::Float(0.0)),
                ("contrast".into(), Value::Float(0.0)),
            ]),
        };
        assert_eq!(inst.type_id, "color_adjust");
        assert_eq!(inst.params.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests and commit**

Run: `cargo test --lib node::registry`

```bash
git add src/node/registry.rs
git commit -m "feat: add NodeInstance struct"
```

---

### Task 14: Implement NodeRegistry

**Files:**
- Modify: `src/node/registry.rs`

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_register_and_query_node() {
    let mut reg = NodeRegistry::new();
    reg.register(NodeDef {
        type_id: "invert".into(),
        title: "Invert".into(),
        category: CategoryId::new("color"),
        inputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: true }],
        outputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: false }],
        params: vec![],
        has_preview: false,
        process: Box::new(|_inputs, _params| HashMap::new()),
    });
    assert!(reg.get("invert").is_some());
    assert_eq!(reg.get("invert").unwrap().title, "Invert");
    assert!(reg.get("nonexistent").is_none());
}

#[test]
fn test_instantiate_node() {
    let mut reg = NodeRegistry::new();
    reg.register(NodeDef {
        type_id: "color_adjust".into(),
        title: "Color Adjustment".into(),
        category: CategoryId::new("color"),
        inputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: true }],
        outputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: false }],
        params: vec![
            ParamDef {
                name: "brightness".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: -1.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Box::new(|_inputs, _params| HashMap::new()),
    });
    let inst = reg.instantiate("color_adjust");
    assert!(inst.is_some());
    let inst = inst.unwrap();
    assert_eq!(inst.type_id, "color_adjust");
    match inst.params.get("brightness") {
        Some(Value::Float(v)) => assert_eq!(*v, 0.0),
        _ => panic!("expected float param"),
    }
}
```

- [ ] **Step 2: Implement NodeRegistry**

```rust
pub struct NodeRegistry {
    nodes: HashMap<String, NodeDef>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    pub fn register(&mut self, def: NodeDef) {
        self.nodes.insert(def.type_id.clone(), def);
    }

    pub fn get(&self, type_id: &str) -> Option<&NodeDef> {
        self.nodes.get(type_id)
    }

    /// Returns all registered nodes, optionally filtered by category.
    pub fn list(&self, category: Option<&CategoryId>) -> Vec<&NodeDef> {
        self.nodes.values()
            .filter(|n| category.map_or(true, |c| n.category == *c))
            .collect()
    }

    /// Creates a new NodeInstance with default parameter values.
    pub fn instantiate(&self, type_id: &str) -> Option<NodeInstance> {
        let def = self.nodes.get(type_id)?;
        let params = def.params.iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect();
        Some(NodeInstance {
            type_id: type_id.to_string(),
            params,
        })
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib node::registry`
Expected: all tests PASS

- [ ] **Step 4: Update mod.rs and commit**

Add to `src/node/mod.rs`:
```rust
pub mod registry;
pub use registry::{PinDef, ParamDef, NodeDef, NodeInstance, NodeRegistry, ProcessFn};
```

```bash
git add src/node/registry.rs src/node/mod.rs
git commit -m "feat: implement NodeRegistry with register/query/instantiate, Chunk 3 complete"
```

---

## Chunk 4: Cache + EvalEngine

### Task 15: Implement Cache

**Files:**
- Create: `src/node/cache.rs`

- [ ] **Step 1: Implement and test**

```rust
// src/node/cache.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::node::types::Value;

pub type NodeId = usize;

pub struct Cache {
    results: HashMap<NodeId, HashMap<String, Value>>,
    /// Downstream adjacency: node → set of nodes that depend on it.
    downstream: HashMap<NodeId, HashSet<NodeId>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            downstream: HashMap::new(),
        }
    }

    pub fn set_downstream(&mut self, from: NodeId, to: NodeId) {
        self.downstream.entry(from).or_default().insert(to);
    }

    pub fn clear_downstream(&mut self) {
        self.downstream.clear();
    }

    pub fn get(&self, node_id: NodeId) -> Option<&HashMap<String, Value>> {
        self.results.get(&node_id)
    }

    pub fn insert(&mut self, node_id: NodeId, outputs: HashMap<String, Value>) {
        self.results.insert(node_id, outputs);
    }

    /// Invalidate this node and all downstream nodes.
    pub fn invalidate(&mut self, node_id: NodeId) {
        self.results.remove(&node_id);
        if let Some(downstream) = self.downstream.get(&node_id).cloned() {
            for d in downstream {
                self.invalidate(d);
            }
        }
    }

    /// Invalidate all cached results.
    pub fn invalidate_all(&mut self) {
        self.results.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = Cache::new();
        let outputs = HashMap::from([("image".into(), Value::Float(1.0))]);
        cache.insert(0, outputs);
        assert!(cache.get(0).is_some());
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn test_local_invalidation() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::new());
        cache.insert(1, HashMap::new());
        cache.insert(2, HashMap::new());

        // 0 → 1 → 2
        cache.set_downstream(0, 1);
        cache.set_downstream(1, 2);

        cache.invalidate(0);
        assert!(cache.get(0).is_none());
        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_partial_invalidation() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::new());
        cache.insert(1, HashMap::new());
        cache.insert(2, HashMap::new());

        // 0 → 1, 0 → 2 (independent branches)
        cache.set_downstream(0, 1);

        cache.invalidate(1);
        assert!(cache.get(0).is_some()); // upstream unaffected
        assert!(cache.get(1).is_none()); // invalidated
        assert!(cache.get(2).is_some()); // independent branch unaffected
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib node::cache`
Expected: 3 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/node/cache.rs
git commit -m "feat: implement Cache with local invalidation"
```

---

### Task 16: Implement EvalEngine

**Files:**
- Create: `src/node/eval.rs`

- [ ] **Step 1: Implement topological sort and evaluation**

```rust
// src/node/eval.rs
use std::collections::{HashMap, HashSet, VecDeque};
use crate::node::types::{Value, DataTypeRegistry};
use crate::node::registry::{NodeRegistry, NodeInstance};
use crate::node::cache::{Cache, NodeId};

pub struct EvalEngine;

/// Represents a graph edge: from output pin to input pin.
pub struct Connection {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}

impl EvalEngine {
    /// Topological sort from a target node, returning nodes in evaluation order.
    /// Returns Err if a cycle is detected.
    pub fn topo_sort(
        target: NodeId,
        connections: &[Connection],
    ) -> Result<Vec<NodeId>, String> {
        // Build reverse adjacency (who feeds into whom)
        let mut deps: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        let mut all_nodes: HashSet<NodeId> = HashSet::new();

        // Collect upstream subgraph from target
        let mut to_visit = VecDeque::new();
        to_visit.push_back(target);
        let mut visited = HashSet::new();

        while let Some(node) = to_visit.pop_front() {
            if !visited.insert(node) { continue; }
            all_nodes.insert(node);
            for conn in connections {
                if conn.to_node == node {
                    deps.entry(node).or_default().insert(conn.from_node);
                    to_visit.push_back(conn.from_node);
                }
            }
        }

        // Kahn's algorithm
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for &n in &all_nodes {
            in_degree.entry(n).or_insert(0);
            if let Some(d) = deps.get(&n) {
                *in_degree.get_mut(&n).unwrap() = d.len();
            }
        }

        let mut queue: VecDeque<NodeId> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&n, _)| n)
            .collect();

        let mut order = Vec::new();
        while let Some(n) = queue.pop_front() {
            order.push(n);
            for (&node, node_deps) in &deps {
                if node_deps.contains(&n) {
                    let deg = in_degree.get_mut(&node).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(node);
                    }
                }
            }
        }

        if order.len() != all_nodes.len() {
            return Err("Cycle detected in node graph".to_string());
        }

        Ok(order)
    }

    /// Evaluate a target node, using cache where possible.
    pub fn evaluate(
        target: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
        type_registry: &DataTypeRegistry,
        cache: &mut Cache,
    ) -> Result<(), String> {
        let order = Self::topo_sort(target, connections)?;

        // Build downstream map for cache
        cache.clear_downstream();
        for conn in connections {
            cache.set_downstream(conn.from_node, conn.to_node);
        }

        for node_id in order {
            // Skip if already cached
            if cache.get(node_id).is_some() {
                continue;
            }

            // Error isolation: skip nodes that can't be evaluated, don't abort the whole graph.
            // This allows independent paths to continue even if one node fails.
            let instance = match nodes.get(&node_id) {
                Some(inst) => inst,
                None => continue, // Node not found — skip
            };
            let def = match node_registry.get(&instance.type_id) {
                Some(d) => d,
                None => continue, // Unknown node type (e.g. unloaded plugin) — skip
            };

            // Gather inputs from upstream connections, applying type conversion if needed
            let mut inputs: HashMap<String, Value> = HashMap::new();
            for conn in connections {
                if conn.to_node == node_id {
                    if let Some(upstream_outputs) = cache.get(conn.from_node) {
                        if let Some(val) = upstream_outputs.get(&conn.from_pin) {
                            // Find the expected input type from the node definition
                            let expected_type = def.inputs.iter()
                                .find(|pin| pin.name == conn.to_pin)
                                .map(|pin| &pin.data_type);

                            // Find the upstream output type
                            let upstream_instance = nodes.get(&conn.from_node);
                            let upstream_type = upstream_instance.and_then(|inst| {
                                node_registry.get(&inst.type_id)
                            }).and_then(|upstream_def| {
                                upstream_def.outputs.iter()
                                    .find(|pin| pin.name == conn.from_pin)
                                    .map(|pin| &pin.data_type)
                            });

                            // Apply type conversion if types differ but are compatible
                            let converted = match (upstream_type, expected_type) {
                                (Some(from_type), Some(to_type)) if from_type != to_type => {
                                    type_registry.convert(val.clone(), from_type, to_type)
                                        .unwrap_or_else(|| val.clone())
                                }
                                _ => val.clone(),
                            };

                            inputs.insert(conn.to_pin.clone(), converted);
                        }
                    }
                }
            }

            // Execute processing function
            let outputs = (def.process)(&inputs, &instance.params);
            cache.insert(node_id, outputs);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topo_sort_linear() {
        // 0 → 1 → 2
        let connections = vec![
            Connection { from_node: 0, from_pin: "out".into(), to_node: 1, to_pin: "in".into() },
            Connection { from_node: 1, from_pin: "out".into(), to_node: 2, to_pin: "in".into() },
        ];
        let order = EvalEngine::topo_sort(2, &connections).unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_topo_sort_diamond() {
        //   0
        //  / \
        // 1   2
        //  \ /
        //   3
        let connections = vec![
            Connection { from_node: 0, from_pin: "out".into(), to_node: 1, to_pin: "in".into() },
            Connection { from_node: 0, from_pin: "out".into(), to_node: 2, to_pin: "in".into() },
            Connection { from_node: 1, from_pin: "out".into(), to_node: 3, to_pin: "in1".into() },
            Connection { from_node: 2, from_pin: "out".into(), to_node: 3, to_pin: "in2".into() },
        ];
        let order = EvalEngine::topo_sort(3, &connections).unwrap();
        assert_eq!(order[0], 0); // must be first
        assert_eq!(order[3], 3); // must be last
    }

    #[test]
    fn test_topo_sort_cycle() {
        // 0 → 1 → 0 (cycle)
        let connections = vec![
            Connection { from_node: 0, from_pin: "out".into(), to_node: 1, to_pin: "in".into() },
            Connection { from_node: 1, from_pin: "out".into(), to_node: 0, to_pin: "in".into() },
        ];
        let result = EvalEngine::topo_sort(0, &connections);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_with_real_registry() {
        // Integration test: build a real NodeRegistry with 2 simple nodes and evaluate a graph.
        use crate::node::types::DataTypeId;
        use crate::node::category::CategoryId;
        use crate::node::registry::{NodeDef, PinDef, ParamDef, NodeRegistry};
        use crate::node::constraint::Constraint;

        let mut node_reg = NodeRegistry::new();
        let type_reg = DataTypeRegistry::with_builtins();

        // Register a "source" node: no inputs, outputs Float value from param
        node_reg.register(NodeDef {
            type_id: "source".into(),
            title: "Source".into(),
            category: CategoryId::new("tool"),
            inputs: vec![],
            outputs: vec![PinDef { name: "value".into(), data_type: DataTypeId::new("float"), required: false }],
            params: vec![ParamDef {
                name: "value".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::None,
                default: Value::Float(0.0),
                widget_override: None,
            }],
            has_preview: false,
            process: Box::new(|_inputs, params| {
                let mut out = HashMap::new();
                if let Some(v) = params.get("value") {
                    out.insert("value".into(), v.clone());
                }
                out
            }),
        });

        // Register an "add_one" node: takes Float input, adds 1.0
        node_reg.register(NodeDef {
            type_id: "add_one".into(),
            title: "Add One".into(),
            category: CategoryId::new("tool"),
            inputs: vec![PinDef { name: "value".into(), data_type: DataTypeId::new("float"), required: true }],
            outputs: vec![PinDef { name: "result".into(), data_type: DataTypeId::new("float"), required: false }],
            params: vec![],
            has_preview: false,
            process: Box::new(|inputs, _params| {
                let mut out = HashMap::new();
                if let Some(Value::Float(v)) = inputs.get("value") {
                    out.insert("result".into(), Value::Float(v + 1.0));
                }
                out
            }),
        });

        // Build graph: source(value=5.0) → add_one
        let mut nodes = HashMap::new();
        nodes.insert(0, NodeInstance {
            type_id: "source".into(),
            params: HashMap::from([("value".into(), Value::Float(5.0))]),
        });
        nodes.insert(1, NodeInstance {
            type_id: "add_one".into(),
            params: HashMap::new(),
        });

        let connections = vec![
            Connection { from_node: 0, from_pin: "value".into(), to_node: 1, to_pin: "value".into() },
        ];

        let mut cache = Cache::new();
        EvalEngine::evaluate(1, &nodes, &connections, &node_reg, &type_reg, &mut cache).unwrap();

        // Check output of add_one node
        let result = cache.get(1).unwrap();
        match result.get("result") {
            Some(Value::Float(v)) => assert_eq!(*v, 6.0),
            other => panic!("expected Float(6.0), got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib node::eval`
Expected: 4 tests PASS

- [ ] **Step 3: Update mod.rs and commit**

Add to `src/node/mod.rs`:
```rust
pub mod cache;
pub mod eval;
```

```bash
git add src/node/eval.rs src/node/cache.rs src/node/mod.rs
git commit -m "feat: implement EvalEngine with topo sort and evaluation, Chunk 4 complete"
```

---

## Chunk 5: Serializer + Menu

### Task 17: Implement Serializer

**Files:**
- Create: `src/node/serial.rs`

- [ ] **Step 1: Define serializable types and implement**

```rust
// src/node/serial.rs
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::node::types::Value;
use crate::node::registry::{NodeInstance, NodeRegistry};

const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct SerializedGraph {
    pub version: u32,
    pub nodes: Vec<SerializedNode>,
    pub connections: Vec<SerializedConnection>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: usize,
    pub type_id: String,
    pub position: [f32; 2],
    pub params: HashMap<String, SerializedValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "value")]
pub enum SerializedValue {
    Float(f32),
    Int(i32),
    Boolean(bool),
    String(String),
    Color([f32; 4]),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedConnection {
    pub from_node: usize,
    pub from_pin: String,
    pub to_node: usize,
    pub to_pin: String,
}

impl SerializedValue {
    pub fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Float(f) => Some(Self::Float(*f)),
            Value::Int(i) => Some(Self::Int(*i)),
            Value::Boolean(b) => Some(Self::Boolean(*b)),
            Value::String(s) => Some(Self::String(s.clone())),
            Value::Color(c) => Some(Self::Color(*c)),
            // Image/Mask are not serializable
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Float(f) => Value::Float(*f),
            Self::Int(i) => Value::Int(*i),
            Self::Boolean(b) => Value::Boolean(*b),
            Self::String(s) => Value::String(s.clone()),
            Self::Color(c) => Value::Color(*c),
        }
    }
}

pub struct Serializer;

impl Serializer {
    pub fn save(graph: &SerializedGraph) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(graph)
    }

    pub fn load(json: &str, registry: &NodeRegistry) -> Result<SerializedGraph, String> {
        let mut graph: SerializedGraph = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Version compat: fill missing params with defaults
        for node in &mut graph.nodes {
            if let Some(def) = registry.get(&node.type_id) {
                for param in &def.params {
                    if !node.params.contains_key(&param.name) {
                        if let Some(sv) = SerializedValue::from_value(&param.default) {
                            node.params.insert(param.name.clone(), sv);
                        }
                    }
                }
            }
            // Unknown node types (e.g. removed plugins) are intentionally kept in the graph
            // so they survive save/load. EvalEngine.evaluate() will skip them because
            // node_registry.get() returns None, and they'll appear as disconnected nodes in the UI.
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_roundtrip() {
        let graph = SerializedGraph {
            version: FORMAT_VERSION,
            nodes: vec![SerializedNode {
                id: 0,
                type_id: "color_adjust".into(),
                position: [100.0, 200.0],
                params: HashMap::from([
                    ("brightness".into(), SerializedValue::Float(0.5)),
                ]),
            }],
            connections: vec![],
        };
        let json = Serializer::save(&graph).unwrap();
        assert!(json.contains("color_adjust"));
        assert!(json.contains("0.5"));
    }

    #[test]
    fn test_serialized_value_conversion() {
        let v = Value::Float(3.14);
        let sv = SerializedValue::from_value(&v).unwrap();
        let v2 = sv.to_value();
        match v2 {
            Value::Float(f) => assert!((f - 3.14).abs() < 0.001),
            _ => panic!("wrong type"),
        }
    }
}
```

- [ ] **Step 2: Run tests and commit**

Run: `cargo test --lib node::serial`

```bash
git add src/node/serial.rs
git commit -m "feat: implement Serializer with version compat"
```

---

### Task 18: Implement Menu

**Files:**
- Create: `src/node/menu.rs`

- [ ] **Step 1: Implement menu generation**

```rust
// src/node/menu.rs
use crate::node::category::{CategoryId, CategoryRegistry};
use crate::node::registry::NodeRegistry;

pub struct MenuItem {
    pub type_id: String,
    pub title: String,
}

pub struct MenuCategory {
    pub id: CategoryId,
    pub name: String,
    pub items: Vec<MenuItem>,
}

pub struct Menu;

impl Menu {
    /// Generate categorized menu from registries.
    pub fn generate(node_reg: &NodeRegistry, cat_reg: &CategoryRegistry) -> Vec<MenuCategory> {
        let mut result = Vec::new();
        for cat in cat_reg.sorted() {
            let nodes = node_reg.list(Some(&cat.id));
            if nodes.is_empty() { continue; }
            result.push(MenuCategory {
                id: cat.id.clone(),
                name: cat.name.clone(),
                items: nodes.iter().map(|n| MenuItem {
                    type_id: n.type_id.clone(),
                    title: n.title.clone(),
                }).collect(),
            });
        }
        result
    }

    /// Filter nodes by search keyword (matches title or category name).
    pub fn search(
        keyword: &str,
        node_reg: &NodeRegistry,
        cat_reg: &CategoryRegistry,
    ) -> Vec<MenuItem> {
        let kw = keyword.to_lowercase();
        node_reg.list(None)
            .iter()
            .filter(|n| {
                n.title.to_lowercase().contains(&kw)
                    || cat_reg.get(&n.category)
                        .map_or(false, |c| c.name.to_lowercase().contains(&kw))
            })
            .map(|n| MenuItem {
                type_id: n.type_id.clone(),
                title: n.title.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::types::{Value, DataTypeId};
    use crate::node::registry::{NodeDef, PinDef, ParamDef};
    use crate::node::constraint::Constraint;

    fn test_registries() -> (NodeRegistry, CategoryRegistry) {
        let cat_reg = CategoryRegistry::with_builtins();
        let mut node_reg = NodeRegistry::new();
        node_reg.register(NodeDef {
            type_id: "invert".into(),
            title: "Invert".into(),
            category: CategoryId::new("color"),
            inputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: true }],
            outputs: vec![PinDef { name: "image".into(), data_type: DataTypeId::new("image"), required: false }],
            params: vec![],
            has_preview: false,
            process: Box::new(|_, _| std::collections::HashMap::new()),
        });
        (node_reg, cat_reg)
    }

    #[test]
    fn test_menu_generation() {
        let (node_reg, cat_reg) = test_registries();
        let menu = Menu::generate(&node_reg, &cat_reg);
        assert_eq!(menu.len(), 1); // only one category has nodes
        assert_eq!(menu[0].name, "颜色处理型");
        assert_eq!(menu[0].items.len(), 1);
    }

    #[test]
    fn test_menu_search() {
        let (node_reg, cat_reg) = test_registries();
        let results = Menu::search("invert", &node_reg, &cat_reg);
        assert_eq!(results.len(), 1);
        let results = Menu::search("nonexistent", &node_reg, &cat_reg);
        assert_eq!(results.len(), 0);
    }
}
```

- [ ] **Step 2: Update mod.rs, run tests, commit**

Add to `src/node/mod.rs`:
```rust
pub mod serial;
pub mod menu;
```

Run: `cargo test --lib node`
Expected: all tests PASS

```bash
git add src/node/serial.rs src/node/menu.rs src/node/mod.rs
git commit -m "feat: implement Serializer and Menu, Chunk 5 complete"
```

---

## Chunk 6: First 4 Builtins

### Task 19: Create builtins module skeleton

**Files:**
- Create: `src/node/builtins/mod.rs`

- [ ] **Step 1: Create mod.rs that will register all builtins**

```rust
// src/node/builtins/mod.rs
mod load_image;
mod color_adjust;
mod preview;
mod save_image;

use crate::node::registry::NodeRegistry;

/// Register all built-in nodes.
pub fn register_all(registry: &mut NodeRegistry) {
    load_image::register(registry);
    color_adjust::register(registry);
    preview::register(registry);
    save_image::register(registry);
}
```

- [ ] **Step 2: Add `pub mod builtins;` to `src/node/mod.rs`**

- [ ] **Step 3: Commit skeleton**

---

### Task 20: Implement LoadImage builtin

**Files:**
- Create: `src/node/builtins/load_image.rs`

- [ ] **Step 1: Implement**

```rust
// src/node/builtins/load_image.rs
use std::sync::Arc;
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId};
use crate::node::constraint::Constraint;
use crate::node::category::CategoryId;
use crate::node::registry::{NodeDef, PinDef, ParamDef, NodeRegistry};

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "load_image".into(),
        title: "Load Image".into(),
        category: CategoryId::new("data"),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![ParamDef {
            name: "path".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::FilePath {
                filters: vec!["png".into(), "jpg".into(), "jpeg".into(), "bmp".into(), "webp".into()],
            },
            default: Value::String(String::new()),
            widget_override: None,
        }],
        has_preview: true,
        process: Box::new(process),
    });
}

fn process(_inputs: &HashMap<String, Value>, params: &HashMap<String, Value>) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::String(path)) = params.get("path") {
        if !path.is_empty() {
            if let Ok(img) = image::open(path) {
                outputs.insert("image".into(), Value::Image(Arc::new(img)));
            }
        }
    }
    outputs
}
```

- [ ] **Step 2: Verify and commit**

Run: `cargo check`

```bash
git add src/node/builtins/load_image.rs
git commit -m "feat: implement LoadImage builtin node"
```

---

### Task 21: Implement ColorAdjust builtin

**Files:**
- Create: `src/node/builtins/color_adjust.rs`

- [ ] **Step 1: Implement (reuse processing.rs)**

```rust
// src/node/builtins/color_adjust.rs
use std::sync::Arc;
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId};
use crate::node::constraint::Constraint;
use crate::node::category::CategoryId;
use crate::node::registry::{NodeDef, PinDef, ParamDef, NodeRegistry};
use crate::processing;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "color_adjust".into(),
        title: "Color Adjustment".into(),
        category: CategoryId::new("color"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![
            ParamDef {
                name: "brightness".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: -1.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "saturation".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: -1.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "contrast".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: -1.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Box::new(process),
    });
}

fn process(inputs: &HashMap<String, Value>, params: &HashMap<String, Value>) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::Image(img)) = inputs.get("image") {
        let b = match params.get("brightness") { Some(Value::Float(v)) => *v, _ => 0.0 };
        let s = match params.get("saturation") { Some(Value::Float(v)) => *v, _ => 0.0 };
        let c = match params.get("contrast") { Some(Value::Float(v)) => *v, _ => 0.0 };
        let result = processing::color_adjust(img, b, c, s);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
```

- [ ] **Step 2: Commit**

```bash
git add src/node/builtins/color_adjust.rs
git commit -m "feat: implement ColorAdjust builtin node"
```

---

### Task 22: Implement Preview and SaveImage builtins

**Files:**
- Create: `src/node/builtins/preview.rs`
- Create: `src/node/builtins/save_image.rs`

- [ ] **Step 1: Implement Preview**

```rust
// src/node/builtins/preview.rs
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId};
use crate::node::category::CategoryId;
use crate::node::registry::{NodeDef, PinDef, NodeRegistry};

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "preview".into(),
        title: "Preview".into(),
        category: CategoryId::new("tool"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![],
        params: vec![],
        has_preview: true,
        process: Box::new(|inputs, _params| {
            // Preview passes the input image through unchanged.
            // The Viewer reads this cached output to render the PreviewArea and update
            // the left-panel preview_texture. No outputs needed since Preview has no
            // output pins, but we store "image" in the result map so the Viewer can
            // retrieve it from Cache for texture upload.
            let mut outputs = HashMap::new();
            if let Some(img) = inputs.get("image") {
                outputs.insert("image".into(), img.clone());
            }
            outputs
        }),
    });
}
```

- [ ] **Step 2: Implement SaveImage**

```rust
// src/node/builtins/save_image.rs
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId};
use crate::node::constraint::Constraint;
use crate::node::category::CategoryId;
use crate::node::registry::{NodeDef, PinDef, ParamDef, NodeRegistry};

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "save_image".into(),
        title: "Save Image".into(),
        category: CategoryId::new("data"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![],
        params: vec![ParamDef {
            name: "path".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::FilePath {
                filters: vec!["png".into(), "jpg".into(), "bmp".into(), "webp".into()],
            },
            default: Value::String(String::new()),
            widget_override: None,
        }],
        has_preview: false,
        process: Box::new(|inputs, params| {
            if let (Some(Value::Image(img)), Some(Value::String(path))) =
                (inputs.get("image"), params.get("path"))
            {
                if !path.is_empty() {
                    let _ = img.save(path);
                }
            }
            HashMap::new()
        }),
    });
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo check`

```bash
git add src/node/builtins/
git commit -m "feat: implement Preview and SaveImage builtins, Chunk 6 complete"
```

---

## Chunk 7: Viewer + App Integration + Migration

### Task 23: Implement generic SnarlViewer

**Files:**
- Create: `src/node/viewer.rs`

- [ ] **Step 1: Implement SnarlViewer for NodeInstance**

This is the most complex file — it reads from all registries to render any node generically. The implementation follows the same pattern as the existing `nodes.rs` but is data-driven.

```rust
// src/node/viewer.rs
use egui::{Color32, Frame, Margin, CornerRadius, Stroke, Ui};
use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, InPinId, OutPin, NodeId, Snarl,
};
use std::collections::HashMap;
use crate::node::types::{Value, DataTypeId, DataTypeRegistry};
use crate::node::constraint::ConstraintType;
use crate::node::category::CategoryRegistry;
use crate::node::registry::{NodeInstance, NodeRegistry};
use crate::node::widget::{WidgetId, WidgetRegistry};
use crate::node::cache::Cache;
use crate::node::menu::Menu;
use crate::node::eval::{EvalEngine, Connection};

const NODE_BG: Color32 = Color32::WHITE;

pub struct NodeViewer {
    pub type_registry: DataTypeRegistry,
    pub category_registry: CategoryRegistry,
    pub node_registry: NodeRegistry,
    pub widget_registry: WidgetRegistry,
    pub cache: Cache,
    /// GPU textures per node. Managed here in Viewer (not in Cache) because TextureHandle
    /// requires egui::Context which is only available during rendering. Cache stays render-agnostic.
    pub textures: HashMap<egui_snarl::NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
}

impl NodeViewer {
    pub fn new() -> Self {
        let mut node_registry = NodeRegistry::new();
        crate::node::builtins::register_all(&mut node_registry);

        Self {
            type_registry: DataTypeRegistry::with_builtins(),
            category_registry: CategoryRegistry::with_builtins(),
            node_registry,
            widget_registry: WidgetRegistry::with_builtins(),
            cache: Cache::new(),
            textures: HashMap::new(),
            preview_texture: None,
        }
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.cache.invalidate(node_id.0);
        self.textures.remove(&node_id);
    }

    pub fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
        self.textures.clear();
    }

    /// Helper: get pin color from type_registry for a given DataTypeId.
    fn pin_color(&self, data_type: &DataTypeId) -> Color32 {
        if let Some(info) = self.type_registry.get(data_type) {
            let [r, g, b] = info.pin_color;
            Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        } else {
            Color32::GRAY
        }
    }

    /// Helper: get or upload texture for a node from cached image data.
    fn get_or_upload_texture(
        &mut self,
        ctx: &egui::Context,
        node_id: NodeId,
        img: &image::DynamicImage,
    ) -> &egui::TextureHandle {
        if !self.textures.contains_key(&node_id) {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
            let tex = ctx.load_texture(
                format!("node-{:?}", node_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );
            self.textures.insert(node_id, tex);
        }
        self.textures.get(&node_id).unwrap()
    }

    /// Build Connection list from the Snarl graph (used by EvalEngine).
    pub fn build_connections(&self, snarl: &Snarl<NodeInstance>) -> Vec<Connection> {
        let mut connections = Vec::new();
        for (node_id, instance) in snarl.node_ids() {
            let def = match self.node_registry.get(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };
            for (i, input_pin) in def.inputs.iter().enumerate() {
                let in_pin = snarl.in_pin(InPinId { node: node_id, input: i });
                for remote in &in_pin.remotes {
                    let upstream_instance = &snarl[remote.node];
                    if let Some(upstream_def) = self.node_registry.get(&upstream_instance.type_id) {
                        if let Some(out_pin) = upstream_def.outputs.get(remote.output) {
                            connections.push(Connection {
                                from_node: remote.node.0,
                                from_pin: out_pin.name.clone(),
                                to_node: node_id.0,
                                to_pin: input_pin.name.clone(),
                            });
                        }
                    }
                }
            }
        }
        connections
    }
}

#[allow(refining_impl_trait)]
impl SnarlViewer<NodeInstance> for NodeViewer {
    fn title(&mut self, node: &NodeInstance) -> String {
        self.node_registry.get(&node.type_id)
            .map(|def| def.title.clone())
            .unwrap_or_else(|| format!("[Unknown: {}]", node.type_id))
    }

    fn inputs(&mut self, node: &NodeInstance) -> usize {
        // V1: only data input pins. TODO: add parameter pins (one per param) for V2.
        self.node_registry.get(&node.type_id)
            .map(|def| def.inputs.len())
            .unwrap_or(0)
    }

    fn outputs(&mut self, node: &NodeInstance) -> usize {
        self.node_registry.get(&node.type_id)
            .map(|def| def.outputs.len())
            .unwrap_or(0)
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
        let instance = &snarl[pin.id.node];
        if let Some(def) = self.node_registry.get(&instance.type_id) {
            if let Some(pin_def) = def.inputs.get(pin.id.input) {
                ui.label(&pin_def.name);
                let color = self.pin_color(&pin_def.data_type);
                return PinInfo::circle().with_fill(color);
            }
        }
        ui.label("?");
        PinInfo::circle().with_fill(Color32::GRAY)
    }

    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
        let instance = &snarl[pin.id.node];
        if let Some(def) = self.node_registry.get(&instance.type_id) {
            if let Some(pin_def) = def.outputs.get(pin.id.output) {
                ui.label(&pin_def.name);
                let color = self.pin_color(&pin_def.data_type);
                return PinInfo::circle().with_fill(color);
            }
        }
        ui.label("?");
        PinInfo::circle().with_fill(Color32::GRAY)
    }

    fn has_body(&mut self, node: &NodeInstance) -> bool {
        self.node_registry.get(&node.type_id)
            .map(|def| !def.params.is_empty() || def.has_preview)
            .unwrap_or(false)
    }

    fn show_body(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        // Clone what we need to avoid borrow conflicts with self
        let instance = snarl[node_id].clone();
        let def = match self.node_registry.get(&instance.type_id) {
            Some(d) => d,
            None => { ui.label("Unknown node type"); return; }
        };
        let params_def: Vec<_> = def.params.clone();
        let has_preview = def.has_preview;

        ui.vertical(|ui| {
            let mut any_changed = false;

            // Render parameter widgets
            for param_def in &params_def {
                ui.horizontal(|ui| {
                    ui.label(&param_def.name);
                    // V1: params don't have input pins, always editable
                    // TODO V2: check if param pin is connected → set disabled=true
                    let disabled = false;
                    let constraint_type = param_def.constraint.constraint_type();

                    if let Some(value) = snarl[node_id].params.get_mut(&param_def.name) {
                        let changed = self.widget_registry.render(
                            param_def.widget_override.as_ref(),
                            &param_def.data_type,
                            &constraint_type,
                            ui,
                            value,
                            &param_def.constraint,
                            &param_def.name,
                            disabled,
                        );
                        if changed {
                            any_changed = true;
                        }
                    }
                });
            }

            if any_changed {
                self.cache.invalidate(node_id.0);
                self.textures.remove(&node_id);
            }

            // Render preview area if enabled
            if has_preview {
                // Evaluate this node to get its output image
                let nodes_map: HashMap<usize, NodeInstance> = snarl.node_ids()
                    .map(|(id, inst)| (id.0, inst.clone()))
                    .collect();
                let connections = self.build_connections(snarl);
                // Explicit field splitting to help the borrow checker see disjoint borrows
                let node_reg = &self.node_registry;
                let type_reg = &self.type_registry;
                let cache = &mut self.cache;
                let _ = EvalEngine::evaluate(
                    node_id.0, &nodes_map, &connections,
                    node_reg, type_reg, cache,
                );

                if let Some(cached) = self.cache.get(node_id.0) {
                    if let Some(Value::Image(img)) = cached.get("image") {
                        // Update left-panel preview texture
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        self.preview_texture = Some(ui.ctx().load_texture(
                            "preview-panel", color_image, egui::TextureOptions::LINEAR,
                        ));

                        // Show thumbnail in node body
                        let tex = self.get_or_upload_texture(ui.ctx(), node_id, img);
                        let tex_size = tex.size_vec2();
                        let scale = (200.0_f32 / tex_size.x.max(tex_size.y)).min(1.0);
                        ui.image(egui::load::SizedTexture::new(tex.id(), tex_size * scale));
                    }
                } else {
                    ui.label("No input connected");
                }
            }
        });
    }

    // ── Frames ──────────────────────────────────────────────────────────

    fn header_frame(
        &mut self,
        _default: Frame,
        _node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        _snarl: &Snarl<NodeInstance>,
    ) -> Frame {
        Frame {
            inner_margin: Margin::symmetric(12, 8),
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
            fill: NODE_BG,
            stroke: Stroke::NONE,
            shadow: egui::Shadow::NONE,
        }
    }

    fn node_frame(
        &mut self,
        _default: Frame,
        _node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        _snarl: &Snarl<NodeInstance>,
    ) -> Frame {
        Frame {
            inner_margin: Margin::same(12),
            outer_margin: Margin::same(0),
            corner_radius: CornerRadius::same(12),
            fill: NODE_BG,
            stroke: Stroke::NONE,
            shadow: egui::Shadow {
                offset: [0, 2],
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(30),
            },
        }
    }

    // ── Menus ───────────────────────────────────────────────────────────

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<NodeInstance>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        let categories = Menu::generate(&self.node_registry, &self.category_registry);
        ui.label("Add Node");
        ui.separator();
        for cat in &categories {
            ui.menu_button(&cat.name, |ui| {
                for item in &cat.items {
                    if ui.button(&item.title).clicked() {
                        if let Some(instance) = self.node_registry.instantiate(&item.type_id) {
                            snarl.insert_node(pos, instance);
                        }
                        ui.close();
                    }
                }
            });
        }
    }

    fn has_node_menu(&mut self, _node: &NodeInstance) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        if ui.button("Delete").clicked() {
            snarl.remove_node(node_id);
            self.invalidate_all();
            ui.close();
        }
    }

    // ── Connections ─────────────────────────────────────────────────────

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        // Type compatibility check
        let from_instance = &snarl[from.id.node];
        let to_instance = &snarl[to.id.node];
        let compatible = (|| {
            let from_def = self.node_registry.get(&from_instance.type_id)?;
            let to_def = self.node_registry.get(&to_instance.type_id)?;
            let from_type = &from_def.outputs.get(from.id.output)?.data_type;
            let to_type = &to_def.inputs.get(to.id.input)?.data_type;
            Some(self.type_registry.is_compatible(from_type, to_type))
        })();

        if compatible != Some(true) {
            return; // Incompatible types — refuse connection
        }

        // Cycle detection: temporarily connect, run topo_sort, revert if cycle found
        // We build the connection list as if this connection exists, then check for cycles
        let would_create_cycle = {
            // Build current connections + proposed new connection
            let mut connections = self.build_connections(snarl);
            // Find pin names for the proposed connection
            let from_pin_name = self.node_registry.get(&snarl[from.id.node].type_id)
                .and_then(|def| def.outputs.get(from.id.output).map(|p| p.name.clone()))
                .unwrap_or_default();
            let to_pin_name = self.node_registry.get(&snarl[to.id.node].type_id)
                .and_then(|def| def.inputs.get(to.id.input).map(|p| p.name.clone()))
                .unwrap_or_default();
            connections.push(Connection {
                from_node: from.id.node.0,
                from_pin: from_pin_name,
                to_node: to.id.node.0,
                to_pin: to_pin_name,
            });
            // Try topo_sort on the target node — if it returns Err, there's a cycle
            EvalEngine::topo_sort(to.id.node.0, &connections).is_err()
        };

        if would_create_cycle {
            return; // Would create cycle — refuse connection
        }

        // Disconnect any existing connection on this input pin
        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl.connect(from.id, to.id);
        self.cache.invalidate(to.id.node.0);
        self.textures.remove(&to.id.node);
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        snarl.disconnect(from.id, to.id);
        self.cache.invalidate(to.id.node.0);
        self.textures.remove(&to.id.node);
    }
}
```

> **Implementation notes:**
> - All rendering is data-driven: node definitions come from `NodeRegistry`, pin colors from `DataTypeRegistry`, widgets from `WidgetRegistry`, and menus from `Menu::generate()`.
> - `connect()` checks type compatibility via `type_registry.is_compatible()` and cycle detection via `EvalEngine::topo_sort()` before allowing a connection. Incompatible or cycle-forming connections are silently rejected.
> - `show_body()` clones the instance to avoid borrow conflicts between `self` (viewer) and `snarl` (graph). This is acceptable for the params HashMap which is small.
> - Parameter pins are deferred to V2 (marked TODO in `inputs()` and `show_body()`). V1 `inputs()` only returns `def.inputs.len()`.
> - The `build_connections()` helper converts egui-snarl's pin-based topology into the `Connection` list expected by `EvalEngine`.
> - Uses `Constraint::constraint_type()` (already implemented in Task 5) to get the `ConstraintType` for widget lookup.

- [ ] **Step 2: Verify compilation**

Run: `cargo check`

- [ ] **Step 3: Commit**

```bash
git add src/node/viewer.rs
git commit -m "feat: implement generic SnarlViewer"
```

---

### Task 24: Update app.rs to use new system

**Files:**
- Modify: `src/app.rs`
- Modify: `src/node/mod.rs`

- [ ] **Step 1: Update imports and App struct**

Replace the `use crate::nodes` import and update the `App` struct:

```rust
// src/app.rs — REPLACE the entire file with:
use eframe::egui;
use egui::Id;
use egui_snarl::Snarl;
use egui_snarl::ui::{SnarlStyle, SnarlWidget, NodeLayout};

use crate::node::viewer::NodeViewer;
use crate::node::registry::NodeInstance;

pub struct App {
    snarl: Snarl<NodeInstance>,  // was: Snarl<Node>
    viewer: NodeViewer,           // was: nodes::NodeViewer
    style: SnarlStyle,
    show_preview: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|s| {
            s.interaction.selectable_labels = false;
        });

        let style = SnarlStyle {
            node_layout: Some(NodeLayout::sandwich()),
            collapsible: Some(false),
            ..SnarlStyle::new()
        };

        Self {
            snarl: Snarl::new(),
            viewer: NodeViewer::new(),
            style,
            show_preview: true,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Left: Preview panel ──
        if self.show_preview {
            let panel_width = ctx.content_rect().width() / 2.0;
            egui::SidePanel::left("preview_panel")
                .default_width(panel_width)
                .min_width(200.0)
                .resizable(true)
                .frame(egui::Frame::NONE.fill(egui::Color32::WHITE))
                .show(ctx, |ui: &mut egui::Ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.add(egui::Button::new("✕").frame(false)).clicked() {
                            self.show_preview = false;
                        }
                    });

                    // Preview texture is set by NodeViewer when a Preview node is evaluated
                    if let Some(tex) = &self.viewer.preview_texture {
                        let size = tex.size_vec2();
                        let available = ui.available_size();
                        let scale = (available.x / size.x)
                            .min(available.y / size.y)
                            .min(1.0);
                        ui.centered_and_justified(|ui: &mut egui::Ui| {
                            ui.image(egui::load::SizedTexture::new(
                                tex.id(),
                                size * scale,
                            ));
                        });
                    }
                });
        }

        // ── Right: Node editor ──
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::WHITE))
            .show(ctx, |ui: &mut egui::Ui| {
                if !self.show_preview {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui.add(egui::Button::new("☐").frame(false)).on_hover_text("Open Preview").clicked() {
                            self.show_preview = true;
                        }
                    });
                }
                SnarlWidget::new()
                    .id(Id::new("node-image-studio"))
                    .style(self.style)
                    .show(&mut self.snarl, &mut self.viewer, ui);
            });
    }
}
```

Summary of changes from the old `app.rs`:
1. `use crate::nodes::{Node, NodeViewer}` → `use crate::node::viewer::NodeViewer` + `use crate::node::registry::NodeInstance`
2. `snarl: Snarl<Node>` → `snarl: Snarl<NodeInstance>`
3. `viewer: NodeViewer` — same field name, but now from `node::viewer` module
4. Preview logic unchanged — `self.viewer.preview_texture` works identically

- [ ] **Step 2: Verify the app runs**

Run: `cargo run --release`
Expected: window opens, node editor works with new system

- [ ] **Step 3: Commit**

```bash
git add src/app.rs src/node/mod.rs
git commit -m "feat: wire app.rs to new node system"
```

---

### Task 25: Remove old nodes.rs

**Files:**
- Delete: `src/nodes.rs`
- Modify: `src/main.rs` (remove `mod nodes;`)

- [ ] **Step 1: Remove old module**

```bash
rm -f src/nodes.rs
```

Remove `mod nodes;` from wherever it's declared.

- [ ] **Step 2: Verify everything compiles and runs**

Run: `cargo check && cargo test --lib node`
Expected: all pass, no references to old module

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "refactor: remove old monolithic nodes.rs, migration complete"
```

---

### Task 26: End-to-end verification

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Manual verification**

Run: `cargo run --release`
Verify:
1. Right-click creates nodes from categorized menu
2. Load Image node loads a file
3. Color Adjustment sliders work
4. Preview node shows image
5. Save Image writes file
6. Connections between nodes work
7. Preview panel shows result

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: node system v1 complete — registry-based architecture"
```
