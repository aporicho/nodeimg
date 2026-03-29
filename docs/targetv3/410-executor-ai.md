# AI 执行器

> 调用 Python 推理后端执行 AI 节点。

## 总览

```mermaid
flowchart TB
    Engine["节点引擎"]

    Engine -->|"内部调度"| AIExec["AI 执行器"]
    AIExec -->|"HTTP + SSE"| Python["Python 推理后端"]
```
