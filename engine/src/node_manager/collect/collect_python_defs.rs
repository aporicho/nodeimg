use crate::node_manager::NodeDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonNodeDeclFormat {
    Decorator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonNodeDeclSpec {
    pub path: &'static str,
    pub format: PythonNodeDeclFormat,
    pub type_id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub inputs: &'static [PythonPinDeclSpec],
    pub outputs: &'static [PythonPinDeclSpec],
    pub params: &'static [PythonParamDeclSpec],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonPinDeclSpec {
    pub name: &'static str,
    pub data_type: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonParamDeclSpec {
    pub name: &'static str,
    pub data_type: &'static str,
    pub default_expr: &'static str,
    pub min_expr: Option<&'static str>,
    pub max_expr: Option<&'static str>,
    pub options_expr: &'static [&'static str],
    pub widget_expr: Option<&'static str>,
    pub expose_expr: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonParamExpose {
    Control,
    Input,
    Output,
}

mod generated {
    use super::{PythonNodeDeclFormat, PythonNodeDeclSpec, PythonParamDeclSpec, PythonPinDeclSpec};

    include!(concat!(env!("OUT_DIR"), "/python_nodes_generated.rs"));
}

pub fn python_node_decl_specs() -> &'static [PythonNodeDeclSpec] {
    generated::PYTHON_NODE_DECL_SPECS
}

pub fn collect_python_defs() -> Vec<NodeDef> {
    Vec::new()
}

fn parse_python_string_literal(expr: &str) -> String {
    if expr.len() >= 2 && expr.starts_with('"') && expr.ends_with('"') {
        expr[1..expr.len() - 1].to_string()
    } else {
        expr.to_string()
    }
}

pub fn parse_param_expose(spec: &PythonParamDeclSpec) -> Vec<PythonParamExpose> {
    spec.expose_expr
        .iter()
        .flat_map(|expr| parse_expose_list_literal(expr))
        .collect()
}

fn parse_expose_list_literal(expr: &str) -> Vec<PythonParamExpose> {
    let trimmed = expr.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Vec::new();
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter_map(|item| match parse_python_string_literal(item).as_str() {
            "control" => Some(PythonParamExpose::Control),
            "input" => Some(PythonParamExpose::Input),
            "output" => Some(PythonParamExpose::Output),
            _ => None,
        })
        .collect()
}
