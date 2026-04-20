use std::collections::HashMap;

use crate::node_manager::{ExecuteFn, NodeDef};
use types::Value;

use super::types::{ExecutionOutputs, ProviderFuture};

pub struct ProviderRequest<'a> {
    pub node_def: &'a NodeDef,
    pub operation: &'a str,
    pub inputs: HashMap<String, Value>,
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn nodes(&self) -> Vec<NodeDef>;
    fn execute<'a>(&'a self, request: ProviderRequest<'a>) -> ProviderFuture<'a>;
}

pub fn placeholder_execute(type_id: &'static str) -> ExecuteFn {
    Box::new(move |_ctx, _inputs| {
        let message = format!("API node '{type_id}' must be executed through ApiExecutor");
        Box::pin(async move { Err(Box::<dyn std::error::Error + Send + Sync>::from(message)) })
    })
}

#[allow(dead_code)]
fn _type_check(_: ExecutionOutputs) {}
