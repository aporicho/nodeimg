use super::{InstanceId, TemplateId, TemplatePayload, TemplateRevision};
use crate::tree::{NodeId, Tree, TreeIndexError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateInstance {
    pub template: TemplateId,
    pub template_revision: TemplateRevision,
    pub instance: InstanceId,
    pub root_node: NodeId,
}

pub trait RetainedTemplate {
    fn id(&self) -> TemplateId;
    fn revision(&self) -> TemplateRevision;

    fn instantiate(
        &self,
        tree: &mut Tree,
        instance: InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<TemplateInstance, TemplateError>;
}

#[derive(Debug)]
pub enum TemplateError {
    MissingTemplate(TemplateId),
    MissingParent(NodeId),
    DuplicateTemplate {
        template: TemplateId,
        existing: TemplateRevision,
        duplicate: TemplateRevision,
    },
    UnsupportedSlot {
        slot: String,
        reason: &'static str,
    },
    UnsupportedPayload {
        template: TemplateId,
        reason: &'static str,
    },
    Index(TreeIndexError),
}

impl Clone for TemplateError {
    fn clone(&self) -> Self {
        match self {
            Self::MissingTemplate(id) => Self::MissingTemplate(id.clone()),
            Self::MissingParent(id) => Self::MissingParent(*id),
            Self::DuplicateTemplate {
                template,
                existing,
                duplicate,
            } => Self::DuplicateTemplate {
                template: template.clone(),
                existing: *existing,
                duplicate: *duplicate,
            },
            Self::UnsupportedSlot { slot, reason } => Self::UnsupportedSlot {
                slot: slot.clone(),
                reason,
            },
            Self::UnsupportedPayload { template, reason } => Self::UnsupportedPayload {
                template: template.clone(),
                reason,
            },
            Self::Index(error) => Self::Index(error.clone()),
        }
    }
}

impl PartialEq for TemplateError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MissingTemplate(a), Self::MissingTemplate(b)) => a == b,
            (Self::MissingParent(a), Self::MissingParent(b)) => a == b,
            (
                Self::DuplicateTemplate {
                    template: template_a,
                    existing: existing_a,
                    duplicate: duplicate_a,
                },
                Self::DuplicateTemplate {
                    template: template_b,
                    existing: existing_b,
                    duplicate: duplicate_b,
                },
            ) => template_a == template_b && existing_a == existing_b && duplicate_a == duplicate_b,
            (
                Self::UnsupportedSlot {
                    slot: slot_a,
                    reason: reason_a,
                },
                Self::UnsupportedSlot {
                    slot: slot_b,
                    reason: reason_b,
                },
            ) => slot_a == slot_b && reason_a == reason_b,
            (
                Self::UnsupportedPayload {
                    template: template_a,
                    reason: reason_a,
                },
                Self::UnsupportedPayload {
                    template: template_b,
                    reason: reason_b,
                },
            ) => template_a == template_b && reason_a == reason_b,
            (Self::Index(a), Self::Index(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for TemplateError {}

impl From<TreeIndexError> for TemplateError {
    fn from(value: TreeIndexError) -> Self {
        Self::Index(value)
    }
}
