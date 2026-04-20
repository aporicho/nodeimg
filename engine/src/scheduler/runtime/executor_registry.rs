use crate::scheduler::model::ExecutorType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutorDescriptor {
    pub executor_type: ExecutorType,
    pub has_gpu: bool,
}

pub struct ExecutorEntry {
    pub name: String,
    pub executor_type: ExecutorType,
    pub can_handle: Box<dyn Fn(ExecutorDescriptor) -> bool + Send + Sync>,
}

impl ExecutorEntry {
    pub fn new(
        name: impl Into<String>,
        executor_type: ExecutorType,
        can_handle: impl Fn(ExecutorDescriptor) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            executor_type,
            can_handle: Box::new(can_handle),
        }
    }

    pub fn matches(&self, descriptor: ExecutorDescriptor) -> bool {
        self.executor_type == descriptor.executor_type && (self.can_handle)(descriptor)
    }
}

#[derive(Default)]
pub struct ExecutorRegistry {
    entries: Vec<ExecutorEntry>,
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, entry: ExecutorEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn resolve(&self, descriptor: ExecutorDescriptor) -> Option<&ExecutorEntry> {
        self.entries.iter().find(|entry| entry.matches(descriptor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_matching_executor_by_type() {
        let mut registry = ExecutorRegistry::new();
        registry.register(ExecutorEntry::new("image", ExecutorType::Image, |_| true));

        let resolved = registry.resolve(ExecutorDescriptor {
            executor_type: ExecutorType::Image,
            has_gpu: false,
        });

        assert_eq!(resolved.map(|entry| entry.name.as_str()), Some("image"));
    }

    #[test]
    fn resolve_returns_none_when_type_does_not_match() {
        let mut registry = ExecutorRegistry::new();
        registry.register(ExecutorEntry::new("image", ExecutorType::Image, |_| true));

        let resolved = registry.resolve(ExecutorDescriptor {
            executor_type: ExecutorType::Ai,
            has_gpu: false,
        });

        assert!(resolved.is_none());
    }

    #[test]
    fn can_handle_allows_same_type_variants() {
        let mut registry = ExecutorRegistry::new();
        registry.register(ExecutorEntry::new(
            "gpu-image",
            ExecutorType::Image,
            |descriptor| descriptor.has_gpu,
        ));
        registry.register(ExecutorEntry::new(
            "cpu-image",
            ExecutorType::Image,
            |descriptor| !descriptor.has_gpu,
        ));

        let gpu_resolved = registry.resolve(ExecutorDescriptor {
            executor_type: ExecutorType::Image,
            has_gpu: true,
        });
        let cpu_resolved = registry.resolve(ExecutorDescriptor {
            executor_type: ExecutorType::Image,
            has_gpu: false,
        });

        assert_eq!(
            gpu_resolved.map(|entry| entry.name.as_str()),
            Some("gpu-image")
        );
        assert_eq!(
            cpu_resolved.map(|entry| entry.name.as_str()),
            Some("cpu-image")
        );
    }
}
