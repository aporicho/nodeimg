# 设计：将序列化职责封装进 Transport trait

> Issue: #73
> 日期: 2026-03-26

## 问题

App 的 Serializer 直接使用 `NodeRegistry`（engine 内部类型）做项目文件的保存/加载。导致：

1. `with_registry()` 是 `LocalTransport` 的具体方法，不在 `ProcessingTransport` trait 上，远程 transport 无法实现
2. App 依赖了 engine 的内部类型，破坏了 Transport 抽象层的隔离
3. 无头运行（nodeimg-server）无法复用序列化逻辑

## 当前状态

Serializer 用 `NodeRegistry` 做三件事：

| 方法 | 用途 | 需要 registry 做什么 |
|------|------|---------------------|
| `load(json, registry)` | JSON → SerializedGraph | 填充缺失参数的默认值 |
| `snapshot(snarl, registry)` | Snarl → SerializedGraph | pin 索引 → 名称转换 |
| `restore(graph, registry)` | SerializedGraph → Snarl | pin 名称 → 索引转换 + 节点实例化 |
| `save(graph)` | SerializedGraph → JSON | 不需要 registry |

App 中有 7 处 `with_registry()` 调用，全部是为了包装 Serializer 调用。

## 方案

### Transport trait 新增 `load_graph` 方法

```rust
trait ProcessingTransport {
    // 现有方法...

    /// 解析项目文件 JSON，填充默认参数值，返回 SerializedGraph。
    fn load_graph(&self, json: &str) -> Result<SerializedGraph, String>;
}
```

`LocalTransport` 用内部 `NodeRegistry` 实现。将来 `HttpTransport` 通过 HTTP 请求远端实现。

这是唯一需要 registry 内部数据参与的"重"操作（填充默认值需要遍历所有节点的 ParamDef）。

### Serializer 改用 `NodeTypeDef`

`snapshot()` 和 `restore()` 的参数从 `&NodeRegistry` 改为 `&HashMap<String, NodeTypeDef>`。

`NodeTypeDef` 已经包含 pin 定义（名称、顺序）和参数默认值，足以完成：
- pin 索引 ↔ 名称转换
- 节点实例化（从 ParamDefInfo 构建默认参数）

节点实例化逻辑（参数默认值只会是标量类型，`ParamValue::to_value()` 不会丢失信息）：
```rust
fn instantiate_from_def(def: &NodeTypeDef) -> NodeInstance {
    NodeInstance {
        type_id: def.type_id.clone(),
        params: def.params.iter()
            .map(|p| (p.name.clone(), p.default.to_value()))
            .collect(),
    }
}
```

`load()` 从 Serializer 中移除，改为 `transport.load_graph()`。
`save()` 保持不变（纯 serde 序列化，不需要 registry）。

### 未知节点的 fallback

当前 `restore()` 在 `registry.instantiate()` 返回 None 时会创建 placeholder NodeInstance，保留图结构完整性。改用 `NodeTypeDef` 后保留相同行为：type_defs 中查不到的类型，直接用保存的参数创建 placeholder（不填默认值）。

### App 缓存节点类型定义

`NodeViewer` 中已有 `node_type_defs: Vec<NodeTypeDef>` 字段（启动时通过 `transport.node_types()` 填充）。将其改为 `HashMap<String, NodeTypeDef>` 并复用，不新增第二个缓存。

## 改动范围

| 文件 | 改动 |
|------|------|
| `crates/nodeimg-engine/src/transport/mod.rs` | `ProcessingTransport` trait 新增 `load_graph` 方法 |
| `crates/nodeimg-engine/src/transport/local.rs` | `LocalTransport` 实现 `load_graph`（搬 Serializer::load 的默认值填充逻辑） |
| `crates/nodeimg-app/src/node/serial.rs` | 移除 `load()`；`snapshot`/`restore` 参数从 `&NodeRegistry` 改为 `&HashMap<String, NodeTypeDef>`；移除 `use nodeimg_engine::NodeRegistry` |
| `crates/nodeimg-app/src/app.rs` | 移除所有 `with_registry()` 调用；加载用 `transport.load_graph()`；snapshot/restore 用缓存的 type_defs；新增 `node_type_defs: HashMap<String, NodeTypeDef>` 字段 |

## 不做什么

- 不新增数据类型（复用现有 `SerializedGraph`、`NodeTypeDef`）
- 不新增 `save_graph` 方法（save 是纯 JSON 序列化，不需要 registry）
- 不移除 `with_registry()`（engine 内部仍需要，只是 app 不再使用）
- 不动 `SerializedGraph` 等类型（已在 `nodeimg-types` 中）
- 不改 `NodeViewer` 中对 `LocalTransport` 具体方法的其他调用（如 `instantiate()`、`generate_menu()`、`register_remote_nodes()`），这些是后续 issue 的范围
