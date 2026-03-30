# 执行调度器

> 拓扑排序，维护脏标记，逐节点分发执行。

## 总览

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    引擎["引擎"]:::external -->|"触发执行"| 调度器["执行调度器"]:::internal

    调度器 --> 规划器["执行规划器"]:::internal
    调度器 --> 运行时["执行运行时"]:::internal
    调度器 --> 取消["取消控制"]:::internal

    节点图["节点图"]:::external -->|"当前/上次版本"| 规划器
    注册表["节点注册表"]:::external -->|"节点定义"| 规划器

    规划器 -->|"分层执行计划"| 运行时

    运行时 <-->|"图像节点"| 图像执行器["图像处理执行器"]:::external
    运行时 <-->|"AI 节点"| AI执行器["AI 执行器"]:::external
    运行时 <-->|"API 节点"| API执行器["API 执行器"]:::external

    运行时 -->|"写入结果"| 缓存
    运行时 -->|"Image 产物"| 产物
    运行时 -->|"进度"| 事件系统["事件系统"]:::external

    取消 -->|"中断执行"| 运行时
    取消 -->|"远程取消"| AI执行器
```

---

## 执行调度器流程

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    触发["引擎触发执行"]:::external
    触发 --> 启动取消["启动取消控制监听"]:::internal
    启动取消 --> 调规划["调用执行规划器"]:::internal
    调规划 --> 有计划{"执行计划为空?"}:::internal
    有计划 -->|"空"| 无需执行["无脏节点，跳过"]:::internal
    有计划 -->|"非空"| 调运行时["调用执行运行时"]:::internal
    调运行时 --> 结果{"执行结果"}:::internal
    结果 -->|"完成"| 推送完成["推送 Finished"]:::internal
    结果 -->|"取消"| 推送取消["推送 Cancelled"]:::internal
    结果 -->|"错误"| 推送错误["推送 Error"]:::internal
    推送完成 --> 清理["清理取消控制"]:::internal
    推送取消 --> 清理
    推送错误 --> 清理
    清理 --> 事件["事件系统"]:::external
```

---

## 执行规划器

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    当前版本["当前图版本"]:::external
    上次版本["上次执行时图版本"]:::external

    当前版本 --> 对比["对比两个版本，找出变化节点"]:::internal
    上次版本 --> 对比

    对比 -->|"变化节点"| 传播["沿拓扑向下游传播脏标记"]:::internal
    节点图["节点图"]:::external -->|"拓扑关系"| 传播

    传播 -->|"脏节点集合"| 排序["拓扑排序，按层分组"]:::internal
    节点图 -->|"全图结构"| 排序

    排序 -->|"分层执行计划"| 输出["执行计划"]:::internal
```

---

## 执行运行时

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    计划["执行计划"]:::external --> 取层["取下一层"]:::internal

    取层 --> 检查取消{"检查取消标志"}:::internal
    检查取消 -->|"已取消"| 终止["终止执行，推送 Cancelled"]:::internal
    检查取消 -->|"继续"| 并行["rayon 同层并行"]:::internal

    并行 --> 收集输入["收集节点输入"]:::internal
    收集输入 --> 分发{"按 executor 分发"}:::internal

    分发 -->|"Image"| 图像执行器["图像执行器"]:::external
    分发 -->|"AI"| AI执行器["AI 执行器"]:::external
    分发 -->|"API"| API执行器["API 执行器"]:::external

    图像执行器 --> 结果判断{"执行结果"}:::internal
    AI执行器 --> 结果判断
    API执行器 --> 结果判断

    结果判断 -->|"成功"| 写结果["写入结果"]:::internal
    结果判断 -->|"失败"| 错误终止["终止执行，推送 Error"]:::internal

    写结果 --> 缓存["缓存管理器"]:::external
    写结果 -->|"Image 产物"| 产物["产物管理器"]:::external
    写结果 --> 上下文["更新执行上下文"]:::internal

    上下文 --> 进度["上报节点完成"]:::internal
    进度 --> 事件["事件系统"]:::external

    进度 --> 还有层{"还有下一层?"}:::internal
    还有层 -->|"是"| 取层
    还有层 -->|"否"| 完成["执行完成，推送 Finished"]:::internal
```

---

## 取消控制

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    用户取消["用户点击取消"]:::external --> 设置标志["设置 CancelToken"]:::internal
    设置标志 --> 运行时检查["运行时在节点边界检查"]:::internal
    运行时检查 --> 是否AI{"当前有 AI 节点在执行?"}:::internal
    是否AI -->|"是"| 远程取消["POST /node/cancel"]:::internal
    是否AI -->|"否"| 停止["停止执行"]:::internal
    远程取消 --> 停止
    停止 --> 推送取消["推送 Cancelled 事件"]:::internal
    推送取消 --> 事件["事件系统"]:::external

    超时启动["节点开始执行，启动超时计时器"]:::internal --> 等待{"等待结果"}:::internal
    等待 -->|"收到 progress"| 重置["重置计时器"]:::internal
    重置 --> 等待
    等待 -->|"超时触发"| 超时取消["调用取消流程"]:::internal
    超时取消 --> 设置标志
    等待 -->|"正常完成"| 清除["清除计时器"]:::internal
```

---

## 组件

- **执行规划器**：对比当前图版本和上次执行时的图版本，找出变化节点，沿拓扑向下游传播脏标记，按层分组输出执行计划。
- **执行运行时**：按执行计划逐层执行，同层节点 rayon 并行。维护执行上下文（中间结果暂存，供下游节点读取输入）。结果写入缓存管理器，AI/API 的 Image 产物额外写入产物管理器。
- **取消控制**：CancelToken（AtomicBool）+ 按节点类型差异化的超时计时器。运行时在节点边界检查标志；AI 节点在 Python 执行时额外发送 POST /node/cancel 远程中断。收到 progress 事件重置超时计时器。

## 边界情况

- **首次执行**：无"上次版本"，所有节点视为脏，全图执行。
- **强制重新执行**（抽卡）：引擎 API 支持 force_execute(node_id)，将指定节点及其下游强制标脏。
- **执行中修改图**：节点图控制器基于不可变状态，调度器持有执行开始时的版本快照，用户修改产生新版本不影响当前执行。
- **断开的子图**：拓扑排序覆盖全图，脏标记只传播到受影响的子图，未变化的子图零开销跳过。

## 设计决策

- **D06**：独立后台线程 + rayon 并行，不引入 async/await。图求值是 CPU/GPU 密集型，线程池模型天然匹配。
- **D07**：AI/API 节点使用 reqwest::blocking 阻塞等待，保持同步执行模型。同层无依赖的 AI 节点通过 rayon 并发发起请求。
- **D23**：前端不阻塞。channel 通信 + ExecuteProgress 事件 + AtomicBool 取消标志。
