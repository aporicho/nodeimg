use super::Graph;
use std::sync::Arc;

pub struct GraphState {
    current: Arc<Graph>,
    undo_stack: Vec<Arc<Graph>>,
    redo_stack: Vec<Arc<Graph>>,
    max_undo: usize,
}

impl GraphState {
    pub fn new(max_undo: usize) -> Self {
        Self {
            current: Arc::new(Graph::new()),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo,
        }
    }

    pub fn current(&self) -> &Graph {
        &self.current
    }

    pub fn snapshot(&self) -> Arc<Graph> {
        Arc::clone(&self.current)
    }

    pub fn commit(&mut self, graph: Graph) {
        self.undo_stack.push(Arc::clone(&self.current));
        if self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.current = Arc::new(graph);
    }

    pub fn preview(&mut self, graph: Graph) {
        self.current = Arc::new(graph);
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(Arc::clone(&self.current));
            self.current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(Arc::clone(&self.current));
            self.current = next;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    pub fn replace(&mut self, graph: Graph) {
        self.current = Arc::new(graph);
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::Vec2;

    #[test]
    fn test_commit_pushes_to_undo_stack() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.commit(g);
        assert!(state.can_undo());
        assert!(!state.can_redo());
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.commit(g);
        assert_eq!(state.current().nodes.len(), 1);
        state.undo();
        assert_eq!(state.current().nodes.len(), 0);
        assert!(state.can_redo());
        state.redo();
        assert_eq!(state.current().nodes.len(), 1);
    }

    #[test]
    fn test_commit_clears_redo() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.commit(g);
        state.undo();
        assert!(state.can_redo());
        let (g, _) = state.current().add_node("b", Vec2::default(), Default::default());
        state.commit(g);
        assert!(!state.can_redo());
    }

    #[test]
    fn test_preview_does_not_push_undo() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.preview(g);
        assert!(!state.can_undo());
        assert_eq!(state.current().nodes.len(), 1);
    }

    #[test]
    fn test_max_undo_depth() {
        let mut state = GraphState::new(2);
        for i in 0..5 {
            let (g, _) = state.current().add_node(&format!("n{i}"), Vec2::default(), Default::default());
            state.commit(g);
        }
        assert!(state.undo());
        assert!(state.undo());
        assert!(!state.undo());
    }

    #[test]
    fn test_replace_clears_stacks() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.commit(g);
        let (g, _) = state.current().add_node("b", Vec2::default(), Default::default());
        state.commit(g);
        state.undo();
        assert!(state.can_undo());
        assert!(state.can_redo());
        state.replace(Graph::new());
        assert!(!state.can_undo());
        assert!(!state.can_redo());
        assert_eq!(state.current().nodes.len(), 0);
    }

    #[test]
    fn test_snapshot_returns_arc_clone() {
        let mut state = GraphState::new(50);
        let (g, _) = state.current().add_node("a", Vec2::default(), Default::default());
        state.commit(g);
        let snap = state.snapshot();
        assert_eq!(snap.nodes.len(), 1);
        // Snapshot is unaffected by subsequent changes
        let (g, _) = state.current().add_node("b", Vec2::default(), Default::default());
        state.commit(g);
        assert_eq!(snap.nodes.len(), 1); // still 1
        assert_eq!(state.current().nodes.len(), 2); // current is 2
    }
}
