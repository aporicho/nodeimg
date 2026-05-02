use super::generated;
use super::{
    CompiledTemplate, InstanceId, RetainedTemplate, SlotValues, TemplateError, TemplateId,
    TemplateInstance, TemplateMountCx, TemplatePayload,
};
use crate::tree::{NodeId, Tree};
use std::collections::HashMap;

#[derive(Default)]
pub struct TemplateRegistry {
    templates: HashMap<TemplateId, CompiledTemplate>,
    retained_templates: HashMap<TemplateId, Box<dyn RetainedTemplate>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_templates() -> Self {
        let mut registry = Self::new();
        generated::register_builtin_templates(&mut registry)
            .expect("builtin templates must not register duplicate incompatible revisions");
        registry
    }

    pub(crate) fn register(&mut self, template: CompiledTemplate) -> Result<(), TemplateError> {
        if let Some(existing) = self.templates.get(&template.id) {
            if existing.revision != template.revision {
                return Err(TemplateError::DuplicateTemplate {
                    template: template.id.clone(),
                    existing: existing.revision,
                    duplicate: template.revision,
                });
            }
        }
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    pub(crate) fn register_retained<T: RetainedTemplate + 'static>(
        &mut self,
        template: T,
    ) -> Result<(), TemplateError> {
        let id = template.id();
        if let Some(existing) = self.retained_templates.get(&id) {
            if existing.revision() != template.revision() {
                return Err(TemplateError::DuplicateTemplate {
                    template: id,
                    existing: existing.revision(),
                    duplicate: template.revision(),
                });
            }
        }
        self.retained_templates.insert(id, Box::new(template));
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn template(&self, template: &TemplateId) -> Option<&CompiledTemplate> {
        self.templates.get(template)
    }

    #[cfg(test)]
    pub(crate) fn instantiate(
        &self,
        tree: &mut Tree,
        template: TemplateId,
        instance: InstanceId,
        parent: NodeId,
        slots: SlotValues,
    ) -> Result<TemplateInstance, TemplateError> {
        self.instantiate_payload(
            tree,
            template,
            instance,
            parent,
            TemplatePayload::Slots(slots),
        )
    }

    pub(crate) fn instantiate_payload(
        &self,
        tree: &mut Tree,
        template: TemplateId,
        instance: InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<TemplateInstance, TemplateError> {
        if tree.get(parent).is_none() {
            return Err(TemplateError::MissingParent(parent));
        }
        if let Some(definition) = self.retained_templates.get(&template) {
            let mut mount = TemplateMountCx::new(tree);
            match definition.instantiate(&mut mount, &instance, parent, payload.clone()) {
                Err(TemplateError::UnsupportedPayload { .. }) => {}
                Err(error) => return Err(error),
                Ok(root) => {
                    return Ok(mount.commit_instance(
                        template.clone(),
                        definition.revision(),
                        instance,
                        root,
                    ));
                }
            }
        }
        let Some(definition) = self.templates.get(&template) else {
            return Err(TemplateError::MissingTemplate(template));
        };
        let Some(slots) = payload.into_slots() else {
            return Err(TemplateError::UnsupportedPayload {
                template,
                reason: "compiled template only accepts slot payload",
            });
        };
        definition.instantiate(tree, instance, parent, slots)
    }

    pub(crate) fn instantiate_root(
        &self,
        tree: &mut Tree,
        template: TemplateId,
        instance: InstanceId,
        slots: SlotValues,
    ) -> Result<TemplateInstance, TemplateError> {
        let Some(definition) = self.templates.get(&template) else {
            return Err(TemplateError::MissingTemplate(template));
        };
        definition.instantiate_root(tree, instance, slots)
    }
}
