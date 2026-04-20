use crate::graph::model::validation::{
    IssueSubject, ValidationIssue, ValidationIssueCode, ValidationReport,
};
use crate::graph::{validate, Graph};
use crate::node_manager::NodeManager;

pub fn validate_graph(graph: &Graph, node_manager: &NodeManager) -> ValidationReport {
    let mut issues = Vec::new();

    for (node_id, node) in &graph.nodes {
        if node_manager.get_node_def(&node.type_id).is_none() {
            issues.push(ValidationIssue {
                code: ValidationIssueCode::MissingNodeType,
                subject: Some(IssueSubject::Node(*node_id)),
                message: format!("Node type '{}' not found", node.type_id),
            });
        }
    }

    for conn in &graph.connections {
        if let Err(error) = validate::validate_connection_basic(graph, conn) {
            issues.push(map_connection_error(conn, error));
            continue;
        }

        if let Err(error) = validate::validate_formal_connection(node_manager, graph, conn) {
            issues.push(map_connection_error(conn, error));
        }
    }

    issues.extend(validate::params::validate_param_values(graph, node_manager));

    ValidationReport::with_issues(issues)
}

fn map_connection_error(
    conn: &crate::graph::Connection,
    error: validate::ConnectionError,
) -> ValidationIssue {
    let code = match error {
        validate::ConnectionError::NodeNotFound(_) => ValidationIssueCode::MissingNode,
        validate::ConnectionError::ConnectionNotFound => ValidationIssueCode::MissingConnection,
        validate::ConnectionError::CycleDetected => ValidationIssueCode::CycleDetected,
        validate::ConnectionError::SelfConnection => ValidationIssueCode::InvalidDirection,
        validate::ConnectionError::InvalidInterface { .. } => ValidationIssueCode::InvalidInterface,
        validate::ConnectionError::InvalidDirection { .. } => ValidationIssueCode::InvalidDirection,
        validate::ConnectionError::TypeIncompatible { .. } => ValidationIssueCode::TypeMismatch,
    };

    ValidationIssue {
        code,
        subject: Some(IssueSubject::Connection {
            from: conn.from.clone(),
            to: conn.to.clone(),
        }),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};
    use crate::node_manager::{NodeDef, ParamDef, ParamExpose, PinDef};
    use types::{Constraint, DataType, Value};

    fn make_manager() -> NodeManager {
        let mut nm = NodeManager::new();
        nm.register(NodeDef {
            type_id: "src".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "src".into(),
            category: "test".into(),
            requires: vec!["test.src".into()],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: Some(Constraint::range(0.0, 1.0)),
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Output],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm.register(NodeDef {
            type_id: "dst".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Builtin,
            name: "dst".into(),
            category: "test".into(),
            requires: vec!["test.dst".into()],
            purity: crate::node_manager::Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: None,
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: Some(Constraint::range(0.0, 1.0)),
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Input],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm
    }

    #[test]
    fn report_ok_for_valid_exposed_connection() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "strength".into(),
            },
            to: PinRef {
                node: b,
                interface: "strength".into(),
            },
        });

        let report = validate_graph(&g, &nm);
        assert!(report.is_valid);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn report_invalid_for_wrong_direction() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "image".into(),
            },
            to: PinRef {
                node: a,
                interface: "image".into(),
            },
        });

        let report = validate_graph(&g, &nm);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == ValidationIssueCode::InvalidDirection));
    }

    #[test]
    fn report_invalid_for_param_constraint_violation() {
        let nm = make_manager();
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let g = g.set_param(a, "strength", Value::Float(2.0));

        let report = validate_graph(&g, &nm);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == ValidationIssueCode::InvalidParamValue));
    }
}
