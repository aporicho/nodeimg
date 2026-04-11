# node_manager

节点管理器（node_manager）负责节点定义的**编译期收集、运行期查询与正式接口展开**。

## 目标

节点管理器只回答一类问题：

- 这个节点是什么
- 这个节点最终暴露了哪些正式接口

它不负责：

- 节点实例增删改
- 连线建立与图校验
- 节点执行
- 项目保存/加载

## 当前结构

```text
node_manager/
├── model/      节点类型模型与 inventory 收集入口
├── collect/    编译期/声明期收集辅助
├── store/      按 type_id 存储 NodeDef
├── index/      分类与搜索索引
└── query/      对外查询接口
```

## 节点来源

### Rust 节点
- 通过 `node!` 宏声明
- 编译期进入 `inventory`

### Python 节点
- 使用 `@node(...)` / `Pin(...)` / `Param(...)` 声明
- `engine/build.rs` 编译期扫描 `python/nodes/*.py`
- 生成 `NodeDefEntry` 注册代码后进入 `inventory`

节点管理器运行时只从统一 `inventory` 收集，不再区分 Rust / Python 两条路径。

## 正式接口视图

节点管理器同时提供两层视图：

1. **原始定义视图**
- `get_node_def`
- `list_node_defs`

2. **展开后的正式接口视图**
- `list_exposed_pins`
- `list_exposed_input_pins`
- `list_exposed_output_pins`
- `get_exposed_pin`
- `get_exposed_input_pin`
- `get_exposed_output_pin`

展开规则：

- 普通 `inputs` / `outputs` 直接进入正式接口视图
- `Param.expose` 会把参数扩展为输入接口或输出接口

## Python 声明校验

`build.rs` 在扫描 Python 节点时，会执行以下规则校验：

- 输出 `Pin(...)` 不允许声明 `required`
- `Param.expose` 必须非空
- `expose` 值只能是 `control` / `input` / `output`
- 复杂类型参数必须使用 `default=None`
- 输入侧接口名必须唯一，输出侧接口名必须唯一
- 允许同名输入与同名输出共存

## 测试覆盖

当前节点管理器测试覆盖包括：

- 基础查询
- Python 节点声明规格生成
- Python 节点编译期注册
- `Param.expose` 解析
- 展开接口视图查询
- Python 声明规则校验的正向/结构性保障
