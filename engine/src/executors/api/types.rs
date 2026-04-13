use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use types::Value;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type ExecutionOutputs = HashMap<String, Value>;
pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExecutionOutputs, BoxError>> + Send + 'a>>;
