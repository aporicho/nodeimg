# 项目管理器

> 管理 .nodeimg bundle 的生命周期，协调序列化器和产物管理器读写项目文件。

## 总览

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    引擎["引擎"]:::external -->|"新建/打开/保存/关闭"| 项目管理器["项目管理器"]:::internal

    项目管理器 --> 序列化器["序列化器"]:::internal

    序列化器 -->|"读写"| graphJson["graph.json"]:::external

    项目管理器 -->|"读写图状态"| 节点图控制器["节点图控制器"]:::external
    项目管理器 -->|"持久化/加载"| 产物管理器["产物管理器"]:::external
```

---

## 新建流程

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    触发["引擎调用新建"]:::external --> 创建目录["创建 .nodeimg 目录"]:::internal
    创建目录 --> 空图["初始化空节点图"]:::internal
    空图 --> 节点图控制器["节点图控制器，重置 undo"]:::external
    创建目录 --> 空产物["初始化空产物管理器"]:::internal
    空产物 --> 产物管理器["产物管理器"]:::external
    创建目录 --> 清缓存["清空缓存管理器"]:::internal
    清缓存 --> 缓存["缓存管理器"]:::external
    节点图控制器 --> 就绪["新项目就绪"]:::internal
    产物管理器 --> 就绪
    缓存 --> 就绪
```

---

## 打开流程

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    触发["引擎调用打开"]:::external --> 读图文件["序列化器读取 graph.json"]:::internal
    读图文件 --> 初始化图["节点图控制器替换图状态，重置 undo"]:::internal
    初始化图 --> 节点图控制器["节点图控制器"]:::external

    触发 --> 加载产物["通知产物管理器加载"]:::internal
    加载产物 --> 产物管理器["产物管理器读取 artifacts.json + 图片目录"]:::external

    初始化图 --> 清缓存["清空缓存管理器"]:::internal
    清缓存 --> 缓存["缓存管理器"]:::external
    清缓存 --> 就绪["项目就绪"]:::internal
    产物管理器 --> 就绪
```

---

## 保存流程

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    触发["引擎调用保存"]:::external --> 读图["从节点图控制器读取图状态"]:::internal
    读图 --> 写图文件["序列化器写入 graph.json"]:::internal
    触发 --> 存产物["通知产物管理器持久化"]:::internal
    存产物 --> 产物管理器["产物管理器写入 artifacts.json + 图片"]:::external
    写图文件 --> 完成["保存完成"]:::internal
    产物管理器 --> 完成
```

---

## 另存为流程

```mermaid
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
    classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff

    触发["引擎调用另存为"]:::external --> 复制["复制 .nodeimg 目录到新路径"]:::internal
    复制 --> 切换["切换当前项目路径"]:::internal
    切换 --> 保存["执行保存流程"]:::internal
```

---

## 操作

| 操作 | 说明 |
|------|------|
| 新建 | 创建空 .nodeimg bundle，初始化空节点图和空产物索引 |
| 打开 | 加载已有 .nodeimg bundle，恢复节点图和产物状态 |
| 保存 | 将当前状态写入 .nodeimg bundle |
| 另存为 | 复制当前项目到新路径，切换到新项目 |
