use gui::tree::Desc;

pub(crate) struct VisualAuditSection {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) column: usize,
    pub(crate) estimated_height: f32,
    pub(crate) content: Vec<Desc>,
}

impl Clone for VisualAuditSection {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: self.title,
            column: self.column,
            estimated_height: self.estimated_height,
            content: self.content.clone(),
        }
    }
}

impl std::fmt::Debug for VisualAuditSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisualAuditSection")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("column", &self.column)
            .field("estimated_height", &self.estimated_height)
            .field("content_len", &self.content.len())
            .finish()
    }
}
