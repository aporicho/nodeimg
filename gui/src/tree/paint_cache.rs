use super::{NodeId, RepaintBoundaryId, Tree};
use crate::paint::PaintFragment;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintCache {
    fragments: HashMap<RepaintBoundaryId, PaintFragment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaintCacheError {
    StaleBoundary { boundary: RepaintBoundaryId },
}

impl PaintCache {
    pub fn get(&self, boundary: RepaintBoundaryId) -> Option<&PaintFragment> {
        self.fragments.get(&boundary)
    }

    pub fn contains(&self, boundary: RepaintBoundaryId) -> bool {
        self.fragments.contains_key(&boundary)
    }

    pub fn insert(&mut self, fragment: PaintFragment) {
        self.fragments.insert(fragment.boundary, fragment);
    }

    pub fn evict(&mut self, boundary: RepaintBoundaryId) -> bool {
        self.fragments.remove(&boundary).is_some()
    }

    pub fn evict_subtree(&mut self, tree: &Tree, root: NodeId) -> usize {
        let mut evicted = 0;
        for boundary in repaint_boundaries_in_subtree(tree, root) {
            if self.evict(boundary) {
                evicted += 1;
            }
        }
        evicted
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn clear(&mut self) {
        self.fragments.clear();
    }

    pub fn validate(&self, tree: &Tree) -> Result<(), PaintCacheError> {
        for boundary in self.fragments.keys().copied() {
            if !tree.is_repaint_boundary(boundary.0) {
                return Err(PaintCacheError::StaleBoundary { boundary });
            }
        }
        Ok(())
    }
}

fn repaint_boundaries_in_subtree(tree: &Tree, root: NodeId) -> Vec<RepaintBoundaryId> {
    let mut boundaries = Vec::new();
    collect_repaint_boundaries(tree, root, &mut boundaries);
    boundaries
}

fn collect_repaint_boundaries(tree: &Tree, node: NodeId, boundaries: &mut Vec<RepaintBoundaryId>) {
    let Some(tree_node) = tree.get(node) else {
        return;
    };
    if tree_node.paint_meta.boundary.is_some() {
        boundaries.push(RepaintBoundaryId(node));
    }
    for child in tree_node.children.iter().copied() {
        collect_repaint_boundaries(tree, child, boundaries);
    }
}
