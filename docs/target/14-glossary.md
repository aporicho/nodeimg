# 术语表

> 项目中使用的领域术语定义。

## 总览

本文件定义 nodeimg 项目中的核心术语，消除歧义。基于现有 domain.md 扩充目标架构新增概念。

## 核心术语

| 术语 | 定义 |
|------|------|
| Node | 节点图中的一个处理单元，有输入引脚、输出引脚和参数 |
| Pin | 节点的输入或输出端口，有名称和数据类型 |
| Connection | 两个节点的引脚之间的数据连接 |
| Graph | 由 Node 和 Connection 组成的有向无环图 |
| Value | 在引脚之间传递的数据值（Image、Float、Int、String、Handle…） |
| Handle | AI 执行器中 Python 端 GPU 对象的引用标识符，不含实际数据 |
| DataType | 引脚或参数的数据类型标识 |
| Constraint | 参数的约束条件（Range、Enum、FilePath…） |
| Transport | 前端与服务层之间的通信抽象层 |
| EvalEngine | 负责拓扑排序和节点执行分发的核心调度器 |
| ExecutionManager | App 逻辑层中管理异步执行任务的组件 |
| ResultCache | 缓存节点执行结果的存储 |
| TextureCache | 缓存预览用 GPU 纹理的存储 |
| ResultEnvelope | 包装执行结果的类型，区分本地引用和远端序列化 |
| NodeDef | 节点类型的元信息定义（引脚、参数、分类） |
| WidgetRegistry | 将 DataType + Constraint 映射为 UI 控件类型的注册表 |
