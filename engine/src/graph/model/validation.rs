use crate::graph::PinRef;
use types::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationIssueCode {
    MissingNode,
    MissingConnection,
    MissingNodeType,
    InvalidParamValue,
    InvalidInterface,
    InvalidDirection,
    TypeMismatch,
    CycleDetected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssueSubject {
    Node(NodeId),
    Connection { from: PinRef, to: PinRef },
    Param { node_id: NodeId, param: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: ValidationIssueCode,
    pub subject: Option<IssueSubject>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            issues: Vec::new(),
        }
    }

    pub fn with_issues(issues: Vec<ValidationIssue>) -> Self {
        Self {
            is_valid: issues.is_empty(),
            issues,
        }
    }
}
