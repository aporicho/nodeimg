# Mermaid 图形规范

> v3 文档体系中所有 mermaid 图的统一风格约定。

## 全局设置

```
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
```

## 形状与颜色

只区分两类：

| 分类 | 形状 | 颜色 | 说明 |
|------|------|------|------|
| 内部模块 | 圆角矩形 `[""]` | 绿 `fill:#6DBFA0` | 当前文档描述的模块自身的组件 |
| 外部模块 | 圆角矩形 `[""]` | 灰 `fill:#B0B8C1` | 不属于当前模块，但有交互的外部依赖 |

```
classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff
```

## 连线

- `-->` 单向
- `<-->` 双向
- `-->|"标签"|` 带标签，2-4 字

## 约束

- 方向 `TB`，曲线 `basis`
- 中文标签，15 字以内
- 单张图不超过 15-20 个节点
