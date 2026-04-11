use crate::executors::image::context::ExecContext;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use types::Value;

use super::{ParamDef, PinDef};

/// 节点执行函数类型。
/// 接收 ExecContext（GPU/CPU 执行上下文）和输入参数，返回输出结果。
pub type ExecuteFn = Box<
    dyn Fn(
            ExecContext<'_>,
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
