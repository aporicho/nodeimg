use crate::capability::CapabilityId;
use crate::executors::image::context::ExecContext;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use types::Value;

use super::{ExecutionPolicy, NodeSourceKind, ParamDef, PinDef, Purity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutorType {
    Image,
    Ai,
    Api,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ApiNodeMeta {
    pub provider_id: String,
    pub operation: String,
}

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
    pub version: u32,
    pub source: NodeSourceKind,
    pub name: String,
    pub category: String,
    pub executor_type: ExecutorType,
    pub requires: Vec<CapabilityId>,
    pub purity: Purity,
    pub cooking_sensitivity: Vec<String>,
    pub realtime_capable: bool,
    pub execution: ExecutionPolicy,
    pub api: Option<ApiNodeMeta>,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub execute: ExecuteFn,
}
