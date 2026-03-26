# 架构路线图

## 阶段 1：清理 Transport 内部

目标：消除 Transport 层的代码重复和职责混乱，无外部影响。

- [ ] #69 — 提取 LocalTransport 中重复的 GraphRequest 准备代码
- [ ] #70 — Transport trait 方法改为接受 `&GraphRequest`
- [ ] #67 — 迁移 WidgetRegistry 出 NodeViewer

三个互相独立，可并行。

## 阶段 2：类型层解耦（为远程执行铺路）

目标：App 对 engine 的直接依赖只剩 `LocalTransport` 一个具体类型。

- [ ] #72 — 将 NodeInstance 从 engine 移到 nodeimg-types
- [ ] #73 — 将序列化职责封装进 Transport trait，移除 app 对 NodeRegistry 的依赖（依赖 #72）

## 阶段 3：远程执行（#12 的实现）

目标：实现 HttpTransport，本地/远端透明切换。

- [ ] #58 — nodeimg-server HTTP 服务实现
- [ ] #12 — HttpTransport 实现，本地/远端透明切换
- [ ] #4 — AI 执行改为异步（可随 server 一起解决）

## 阶段 4：长期演进

- [ ] #5 — V2 参数 Pin 连接支持
- [ ] #14 + #15 — UI 框架从 egui 迁移到 iced + 自建节点图编辑器
- [ ] #7 — 整理文件架构（被以上重构逐步消化，阶段 2 后评估是否还需要）
