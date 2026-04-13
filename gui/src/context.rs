use crate::gesture::GestureArena;
use crate::renderer::{Rect, Renderer, TextMeasurer};
use crate::tree::{hit_test, layout, paint, reconcile, Desc, HitChain, NodeId, Tree};

/// GUI 中心对象。持有统一的控件树与当前手势竞技场。
pub struct Context {
    tree: Tree,
    gesture_arena: Option<GestureArena>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            gesture_arena: None,
        }
    }

    /// 更新整棵树并重新布局。
    pub fn update(&mut self, desc: Desc, root_rect: Rect, measurer: &mut TextMeasurer) {
        reconcile(&mut self.tree, desc);
        if let Some(root) = self.tree.root() {
            layout(&mut self.tree, root, root_rect, &mut |text, size| {
                measurer.measure(text, size)
            });
        }
    }

    /// 渲染整棵树。
    pub fn render(&self, renderer: &mut Renderer, _viewport_w: f32, _viewport_h: f32) {
        if let Some(root) = self.tree.root() {
            paint(&self.tree, root, renderer);
        }
    }

    /// 命中测试，返回从叶子到根的命中链。
    pub fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test(&self.tree, root, x, y)
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn gesture_arena(&self) -> Option<&GestureArena> {
        self.gesture_arena.as_ref()
    }

    pub fn gesture_arena_mut(&mut self) -> Option<&mut GestureArena> {
        self.gesture_arena.as_mut()
    }

    pub fn set_gesture_arena(&mut self, arena: GestureArena) {
        self.gesture_arena = Some(arena);
    }

    pub fn clear_gesture_arena(&mut self) {
        self.gesture_arena = None;
    }

    pub fn take_gesture_arena(&mut self) -> Option<GestureArena> {
        self.gesture_arena.take()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
