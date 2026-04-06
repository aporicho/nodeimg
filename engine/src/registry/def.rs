use types::{Constraint, DataType, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// 引脚定义（输入/输出）。
pub struct PinDef {
    pub name: String,
    pub data_type: DataType,
    pub optional: bool,
}

/// 参数定义。
pub struct ParamDef {
    pub name: String,
    pub data_type: DataType,
    pub constraint: Option<Constraint>,
    pub default_value: Value,
}

/// 节点执行函数类型。
/// 注意：ExecContext 在 Task 10 才实现，这里先用泛型占位。
/// 实际签名是 Fn(ExecContext, HashMap) -> Future<Result<HashMap>>。
/// 暂时定义为接收 HashMap 参数返回 HashMap 结果的 async 函数。
pub type ExecuteFn = Box<
    dyn Fn(
            HashMap<String, Value>,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            HashMap<String, Value>,
                            Box<dyn std::error::Error + Send + Sync>,
                        >,
                    > + Send,
            >,
        > + Send
        + Sync,
>;

/// 节点类型定义。描述一个节点的输入、输出、参数和执行函数。
pub struct NodeDef {
    pub type_id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub execute: ExecuteFn,
}
