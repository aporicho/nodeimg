# 图像处理执行器

> 定位：本地 GPU + CPU 协作的图像处理执行路径——像素运算走 GPU shader，文件 I/O 和分析走 CPU。

## 架构总览

```mermaid
flowchart TD
    subgraph 图像处理执行器["图像处理执行器（同进程，无网络调用）"]
        INPUT["EvalEngine 分发的节点\nexecutor == ExecutorType::Image"]:::service

        DISPATCH{"分派决策\ngpu_process 优先\nprocess 回退"}:::foundation

        subgraph GPU路径["GPU 路径（gpu_process: Some）"]
            direction TB
            G1["shader 源码\ninclude_str! 编译期嵌入\n跟随节点文件夹存放"]:::compute
            G2["nodeimg-gpu 运行时\nGpuContext · pipeline 管理"]:::compute
            G3["16×16 workgroup 分发\n像素级并行计算"]:::compute
            G1 --> G2 --> G3
        end

        subgraph CPU路径["CPU 路径（process: Some）"]
            direction TB
            C1["process 函数\n&[u8] / DynamicImage"]:::compute
            C2["nodeimg-processing 算法\nhistogram · LUT 解析"]:::compute
            C3["文件 I/O\nload_image · save_image"]:::compute
            C1 --> C2
            C1 --> C3
        end

        OUTPUT_G["Value::GpuImage\n→ ResultCache"]:::service
        OUTPUT_C["Value::Image\n→ ResultCache"]:::service

        INPUT --> DISPATCH
        DISPATCH -->|"gpu_process 优先"| GPU路径
        DISPATCH -->|"process（GPU 不可用时降级）"| CPU路径
        G3 --> OUTPUT_G
        C1 --> OUTPUT_C
    end

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

## GPU 路径

**GPU 路径（`gpu_process: Some`）— 像素级运算**

执行器从节点定义中取出 shader 源码（通过 `include_str!` 在编译期嵌入，跟随节点文件夹存放），提交给 `nodeimg-gpu` 运行时，按 `16×16` workgroup size 分发计算。所有像素级运算（亮度、对比度、模糊、混合等）走此路径。

## CPU 路径

**CPU 路径（`process: Some`）— 文件 I/O 与数据分析**

节点函数直接以 `&[u8]`（或 `image::DynamicImage`）为输入输出，调用 `nodeimg-processing` 中的算法。适用于 GPU 无法完成的操作：文件 I/O（`load_image`、`save_image`）、直方图计算、LUT 文件解析。

## 分派规则

大多数节点只提供一条路径——像素运算只写 `gpu_process`，I/O 和分析只写 `process`。少数节点（如 `gaussian_blur`）同时提供两条路径，此时 GPU 优先，CPU 仅在 GPU 上下文不可用时（无兼容 GPU 或驱动问题）作为降级选项。

## 设计决策

- **D29**：GPU 优先，CPU 仅在 GPU 不可用时降级
