# cache

`engine/src/cache/` 是引擎内部的缓存管理模块。

它负责三类能力：

1. **存取**：缓存节点输出结果与预览纹理。
2. **失效**：在节点、输出、子图或 generation 变化时清理旧条目。
3. **淘汰**：在预算约束下按 LRU 回收结果缓存与纹理缓存。

它**不负责**：

- 调度器脏传播；
- artifact 回源决策；
- 项目生命周期管理。

---

## 模块结构

```text
cache/
├── mod.rs
├── README.md
├── manager.rs
├── model/
├── result_store/
├── texture_store/
└── release_queue/
```

### `manager.rs`

缓存管理器门面，对外暴露统一接口：

- `get_result(...)`
- `put_result(...)`
- `put_result_with_policy(...)`
- `get_preview_texture(...)`
- `invalidate_output(...)`
- `invalidate_node(...)`
- `invalidate_subgraph(...)`
- `clear_all()`
- `current_generation()`
- `clear_handles()`
- `stats_snapshot()`
- `events_snapshot()`
- `retry_pending_release(...)`

### `model/`

共享模型：

- 键：`CacheKey` / `TextureKey`
- 签名：`ExecSignature`
- 代际：`GenerationId`
- 请求：`PreviewRequest`
- 条目：`ResultEntry` / `TextureEntry`
- 预算：`CacheBudget`
- 错误：`CacheError`
- 统计：`CacheStats`
- 事件：`CacheEvent`
- 策略：`CachePolicy`

### `result_store/`

结果缓存：

- 查询 / 写入 / 失效
- 结果字节估算
- 结果 LRU
- 结果预算淘汰

### `texture_store/`

纹理缓存：

- 纹理查询 / 写入 / 失效
- 预览源选择
- `Value::Image -> GpuTexture`
- 纹理 LRU
- 纹理预算淘汰

### `release_queue/`

Handle 生命周期补偿：

- `pending_release` 队列
- 重试策略
- 单步重试驱动

---

## 当前实现状态

当前已经完成：

- 结果缓存与纹理缓存分离
- generation 防回灌
- 结果 / 纹理 / Handle 三桶预算
- Handle 不进入主 LRU
- Handle 累计预算检查
- 覆盖旧 Handle / 淘汰 Handle / 过期 Handle 入释放队列
- `clear_handles()` 本地兜底清理
- 释放重试驱动
- 计数统计与事件流
- 最小外部接线：`Engine::evaluate()` 已支持最小缓存命中与写回

当前仍属**最小可用版**，后续可继续增强：

- 完整 `ExecSignature` 规范化生成器
- artifact 回源链路
- scheduler 失效链路
- 更完整的事件消费 / 日志接线

---

## 预算语义

预算分三桶：

- `result_budget_bytes`
- `texture_budget_bytes`
- `handle_budget_bytes`

规则：

- 普通结果进入结果缓存预算；
- 预览纹理进入纹理缓存预算；
- Handle 使用独立预算，不参与主 LRU；
- 超 `soft` 记录压力并收缩；
- Handle 超 `hard` 会拒绝新增；
- 新增 Handle 前优先清理过期 Handle。

---

## 可观测性

当前支持两类观测：

1. **统计快照**：`stats_snapshot()`
2. **事件快照**：`events_snapshot()`

事件覆盖：

- 压力事件
- generation 丢弃写入
- Handle 释放结果
- 预览未命中

---

## 测试

当前缓存相关单测覆盖：

- 模型层
- 结果缓存层
- 纹理缓存层
- Handle 预算与生命周期
- release_queue 重试
- CacheManager 统计 / 事件 / generation / 淘汰

运行方式：

```bash
cargo test -p engine cache:: --lib
```
