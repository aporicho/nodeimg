use crate::graph::model::validation::{IssueSubject, ValidationIssue, ValidationIssueCode};
use crate::graph::Graph;
use crate::node_manager::NodeManager;
use types::{Constraint, Value};

pub fn validate_param_values(graph: &Graph, node_manager: &NodeManager) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (node_id, node) in &graph.nodes {
        let Some(_def) = node_manager.get_node_def(&node.type_id) else {
            continue;
        };

        let resolved_schema = match node_manager.resolve_schema(&node.type_id, &node.params) {
            Ok(schema) => schema,
            Err(error) => {
                issues.push(ValidationIssue {
                    code: ValidationIssueCode::InvalidParamValue,
                    subject: Some(IssueSubject::Node(*node_id)),
                    message: error.to_string(),
                });
                continue;
            }
        };

        for param_def in &resolved_schema.params {
            let Some(value) = node.params.get(&param_def.name) else {
                continue;
            };

            if let Some(constraint) = &param_def.constraint {
                if let Some(message) = validate_value_against_constraint(value, constraint) {
                    issues.push(ValidationIssue {
                        code: ValidationIssueCode::InvalidParamValue,
                        subject: Some(IssueSubject::Param {
                            node_id: *node_id,
                            param: param_def.name.clone(),
                        }),
                        message,
                    });
                }
            }
        }
    }

    issues
}

fn validate_value_against_constraint(value: &Value, constraint: &Constraint) -> Option<String> {
    match constraint.type_id.as_str() {
        "range" => validate_range(value, constraint),
        "enum" => validate_enum(value, constraint),
        "file_path" => validate_file_path(value, constraint),
        _ => None,
    }
}

fn validate_range(value: &Value, constraint: &Constraint) -> Option<String> {
    let min = constraint.params.get("min")?.as_f64()?;
    let max = constraint.params.get("max")?.as_f64()?;
    let numeric = match value {
        Value::Float(v) => *v as f64,
        Value::Int(v) => *v as f64,
        _ => return Some(format!("value is not numeric for range [{}, {}]", min, max)),
    };

    if numeric < min || numeric > max {
        Some(format!(
            "value {} is outside range [{}, {}]",
            numeric, min, max
        ))
    } else {
        None
    }
}

fn validate_enum(value: &Value, constraint: &Constraint) -> Option<String> {
    let options = constraint.params.get("options")?.as_array()?;
    let string = match value {
        Value::String(v) => v,
        _ => return Some("value is not a string for enum constraint".into()),
    };

    let matched = options
        .iter()
        .filter_map(|item| item.as_str())
        .any(|item| item == string);
    if matched {
        None
    } else {
        Some(format!("value '{}' is not in enum options", string))
    }
}

fn validate_file_path(value: &Value, constraint: &Constraint) -> Option<String> {
    let extensions = constraint.params.get("extensions")?.as_array()?;
    let path = match value {
        Value::String(v) => v,
        _ => return Some("value is not a string for file_path constraint".into()),
    };

    let matched = extensions
        .iter()
        .filter_map(|item| item.as_str())
        .any(|ext| path.ends_with(ext));
    if matched || path.is_empty() {
        None
    } else {
        Some(format!("path '{}' does not match allowed extensions", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::{NodeDef, NodeManager, ParamDef};
    use types::{DataType, Value};

    fn make_manager() -> NodeManager {
        let mut nm = NodeManager::new();
        nm.register(NodeDef {
            type_id: "test".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "test".into(),
            category: "test".into(),
            requires: vec!["test.node".into()],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![],
            outputs: vec![],
            params: vec![
                ParamDef {
                    name: "amount".into(),
                    data_type: DataType::float(),
                    constraint: Some(Constraint::range(0.0, 1.0)),
                    default_value: Value::Float(0.5),
                    expose: vec![],
                },
                ParamDef {
                    name: "mode".into(),
                    data_type: DataType::string(),
                    constraint: Some(Constraint::enum_options(vec!["a".into(), "b".into()])),
                    default_value: Value::String("a".into()),
                    expose: vec![],
                },
            ],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm
    }

    #[test]
    fn reports_invalid_range_param() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, id) = g.add_node("test", Default::default());
        let g = g.set_param(id, "amount", Value::Float(2.0));
        let issues = validate_param_values(&g, &nm);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, ValidationIssueCode::InvalidParamValue);
    }

    #[test]
    fn reports_invalid_enum_param() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, id) = g.add_node("test", Default::default());
        let g = g.set_param(id, "mode", Value::String("c".into()));
        let issues = validate_param_values(&g, &nm);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, ValidationIssueCode::InvalidParamValue);
    }
}
