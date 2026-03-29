# API 执行器

> 调用云端模型 API 执行推理。

## 总览

```mermaid
flowchart TB
    Engine["节点引擎"]

    Engine -->|"内部调度"| APIExec["API 执行器"]
    APIExec -->|"HTTPS REST"| Cloud["云端模型 API"]
```
