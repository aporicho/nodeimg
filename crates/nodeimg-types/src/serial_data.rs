use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct SerializedGraph {
    pub version: u32,
    pub nodes: Vec<SerializedNode>,
    pub connections: Vec<SerializedConnection>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: usize,
    pub type_id: String,
    pub position: [f32; 2],
    pub params: HashMap<String, SerializedValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "value")]
pub enum SerializedValue {
    Float(f32),
    Int(i32),
    Boolean(bool),
    String(String),
    Color([f32; 4]),
}

#[derive(Serialize, Deserialize)]
pub struct SerializedConnection {
    pub from_node: usize,
    pub from_pin: String,
    pub to_node: usize,
    pub to_pin: String,
}

impl SerializedValue {
    pub fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Float(f) => Some(Self::Float(*f)),
            Value::Int(i) => Some(Self::Int(*i)),
            Value::Boolean(b) => Some(Self::Boolean(*b)),
            Value::String(s) => Some(Self::String(s.clone())),
            Value::Color(c) => Some(Self::Color(*c)),
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Float(f) => Value::Float(*f),
            Self::Int(i) => Value::Int(*i),
            Self::Boolean(b) => Value::Boolean(*b),
            Self::String(s) => Value::String(s.clone()),
            Self::Color(c) => Value::Color(*c),
        }
    }
}
