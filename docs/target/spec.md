# 文档规范

> targetv3 文档体系的编写约定。

---

## 文件命名

### 编号格式

```
x.x.x-name.md
```

| 层级 | 格式 | 示例 |
|------|------|------|
| 顶层模块 | `x.0.0` | `2.0.0-gui.md` |
| 子模块 | `x.x.0` | `2.1.0-canvas.md` |
| 子子模块 | `x.x.x` | `4.11.1-provider.md` |

无编号的文件为全局文档（如 `spec.md`、`README.md`），不属于任何模块层级。

### 状态后缀

在编号和文件扩展名之间加状态标注：

```
x.x.x-name.{status}.md
```

| 后缀 | 含义 |
|------|------|
| `.impl` | 规格成熟，待代码实现 |
| `.future` | 规划中，未来实现 |
| `.draft` | 设计未成熟，仍在讨论 |

无状态后缀 = 总览 / 索引类文档，或暂未标注。

示例：
```
2.1.0-canvas.impl.md
4.11.1-provider.future.md
2.9.0-canvas-controller.draft.md
```

---

## Mermaid 图规范

### 全局设置

```
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
```

### 节点分类

| 分类 | 颜色 | 说明 |
|------|------|------|
| 内部模块 | 绿 `fill:#6DBFA0` | 当前文档描述的模块自身的前端组件 |
| 逻辑模块 | 蓝 `fill:#5B9BD5` | 当前文档描述的模块自身的后端 / 逻辑层组件 |
| 外部模块 | 灰 `fill:#B0B8C1` | 不属于当前模块，但有交互的外部依赖 |

```
classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
classDef logic fill:#5B9BD5,stroke:#4A8AC4,color:#fff
classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff
```

### 连线

- `-->` 单向
- `<-->` 双向
- `-->|"标签"|` 带标签，2–4 字

### 约束

- 方向 `TB`，曲线 `basis`
- 中文标签，15 字以内
- 单张图不超过 15–20 个节点
