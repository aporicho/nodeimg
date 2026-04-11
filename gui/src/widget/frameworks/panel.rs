use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::tree::Desc;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind, Position, Size};
use crate::widget::props::{WidgetBuild, WidgetProps};

/// 标题栏固定高度（像素）
pub const TITLE_BAR_HEIGHT: f32 = 32.0;

// 占位装饰（C.7 接主题系统后替换）
fn frame_bg()     -> Color { Color { r: 0.133, g: 0.145, b: 0.196, a: 1.0 } }
fn frame_border() -> Color { Color { r: 0.263, g: 0.278, b: 0.333, a: 1.0 } }
fn titlebar_bg()  -> Color { Color { r: 0.180, g: 0.196, b: 0.255, a: 1.0 } }
fn title_color()  -> Color { Color { r: 0.902, g: 0.910, b: 0.941, a: 1.0 } }
const FRAME_RADIUS:    f32 = 6.0;
const TITLE_FONT_SIZE: f32 = 13.0;

/// Panel widget 的 props。
///
/// Controlled 模型：x/y/w/h 每帧由调用方传入，widget 本身不持任何状态。
/// 拖拽/resize 手势产生的 Action 由调用方处理并更新 props。
pub struct PanelProps {
    pub id: Cow<'static, str>,
    pub title: Cow<'static, str>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub content: Vec<Desc>,
}

impl Clone for PanelProps {
    fn clone(&self) -> Self {
        PanelProps {
            id: self.id.clone(),
            title: self.title.clone(),
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            content: self.content.iter().map(desc_clone).collect(),
        }
    }
}

fn desc_clone(d: &Desc) -> Desc {
    match d {
        Desc::Container { id, style, decoration, children } => Desc::Container {
            id: id.clone(),
            style: style.clone(),
            decoration: decoration.clone(),
            children: children.iter().map(desc_clone).collect(),
        },
        Desc::Leaf { id, style, kind } => Desc::Leaf {
            id: id.clone(),
            style: style.clone(),
            kind: kind.clone(),
        },
        Desc::Widget { id, props } => Desc::Widget {
            id: id.clone(),
            props: props.clone_box(),
        },
    }
}

impl fmt::Debug for PanelProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PanelProps")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("w", &self.w)
            .field("h", &self.h)
            .field("content_len", &self.content.len())
            .finish()
    }
}

impl WidgetProps for PanelProps {
    fn widget_type(&self) -> &'static str {
        "Panel"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|o| {
                self.id == o.id
                    && self.title == o.title
                    && self.x == o.x
                    && self.y == o.y
                    && self.w == o.w
                    && self.h == o.h
            })
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str) -> WidgetBuild {
        // 标题栏
        let titlebar = Desc::Container {
            id: Cow::Owned(format!("{id}::titlebar")),
            style: BoxStyle {
                height: Size::Fixed(TITLE_BAR_HEIGHT),
                padding: Edges::symmetric(6.0, 10.0),
                direction: Direction::Row,
                gestures: vec![Gesture::Drag],
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(titlebar_bg()),
                border: None,
                radius: [FRAME_RADIUS, FRAME_RADIUS, 0.0, 0.0],
                shadow: None,
            }),
            children: vec![
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::title")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.title.to_string(),
                        font_size: TITLE_FONT_SIZE,
                        color: title_color(),
                    },
                },
            ],
        };

        // 内容区（透明，事件穿透到子控件）
        let content_area = Desc::Container {
            id: Cow::Owned(format!("{id}::content")),
            style: BoxStyle {
                flex_grow: 1.0,
                padding: Edges::all(8.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: self.content.iter().map(desc_clone).collect(),
        };

        WidgetBuild {
            style: BoxStyle {
                position: Position::Absolute { x: self.x, y: self.y },
                width: Size::Fixed(self.w),
                height: Size::Fixed(self.h),
                direction: Direction::Column,
                gestures: vec![Gesture::Resize],
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(frame_bg()),
                border: Some(Border {
                    width: 1.0,
                    color: frame_border(),
                }),
                radius: [FRAME_RADIUS; 4],
                shadow: None,
            }),
            children: vec![titlebar, content_area],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gesture::Gesture;
    use crate::renderer::{Color, Rect};
    use crate::tree::layout::{LeafKind, Position, Size};
    use crate::tree::{hit_test, layout, reconcile, NodeId, Tree};

    /// 标准 props：x=10, y=20, w=300, h=200，空 content。
    fn sample_props() -> PanelProps {
        PanelProps {
            id: Cow::Borrowed("test"),
            title: Cow::Borrowed("Title"),
            x: 10.0,
            y: 20.0,
            w: 300.0,
            h: 200.0,
            content: Vec::new(),
        }
    }

    /// 集成测试 helper：把 PanelProps 装进树，走 reconcile + layout，
    /// 返回可直接 hit_test 的 Tree + root NodeId。
    fn build_tree_for_hit(props: PanelProps) -> (Tree, NodeId) {
        let desc = Desc::Widget {
            id: Cow::Borrowed("test_panel"),
            props: Box::new(props),
        };

        let mut tree = Tree::new();
        reconcile(&mut tree, desc);

        let root = tree.root().expect("tree should have root after reconcile");

        let mut no_measure = |_text: &str, _size: f32| -> (f32, f32) { (0.0, 0.0) };
        layout(
            &mut tree,
            root,
            Rect { x: 0.0, y: 0.0, w: 1000.0, h: 1000.0 },
            &mut no_measure,
        );

        (tree, root)
    }

    fn desc_variant_name(d: &Desc) -> &'static str {
        match d {
            Desc::Container { .. } => "Container",
            Desc::Leaf { .. } => "Leaf",
            Desc::Widget { .. } => "Widget",
        }
    }

    // ── 结构测试 ──

    #[test]
    fn widget_type_is_panel() {
        assert_eq!(sample_props().widget_type(), "Panel");
    }

    #[test]
    fn build_outer_is_absolute() {
        let build = sample_props().build("test");
        match build.style.position {
            Position::Absolute { x, y } => {
                assert_eq!(x, 10.0);
                assert_eq!(y, 20.0);
            }
            other => panic!("expected Absolute, got {:?}", other),
        }
        match build.style.width {
            Size::Fixed(v) => assert_eq!(v, 300.0),
            other => panic!("expected Fixed width, got {:?}", other),
        }
        match build.style.height {
            Size::Fixed(v) => assert_eq!(v, 200.0),
            other => panic!("expected Fixed height, got {:?}", other),
        }
    }

    #[test]
    fn build_outer_has_resize_gesture() {
        let build = sample_props().build("test");
        assert_eq!(build.style.gestures, vec![Gesture::Resize]);
    }

    #[test]
    fn build_outer_has_frame_decoration() {
        let build = sample_props().build("test");
        let dec = build.decoration.expect("outer should have decoration");
        assert!(dec.background.is_some(), "outer should have background");
        assert!(dec.border.is_some(), "outer should have border");
    }

    #[test]
    fn build_children_count_is_two() {
        let build = sample_props().build("test");
        assert_eq!(build.children.len(), 2, "panel should have titlebar + content");
    }

    #[test]
    fn build_titlebar_has_drag_gesture() {
        let build = sample_props().build("test");
        match &build.children[0] {
            Desc::Container { style, .. } => {
                assert_eq!(style.gestures, vec![Gesture::Drag]);
                match style.height {
                    Size::Fixed(h) => assert_eq!(h, TITLE_BAR_HEIGHT),
                    other => panic!("expected Fixed titlebar height, got {:?}", other),
                }
            }
            other => panic!("first child should be Container, got {}", desc_variant_name(other)),
        }
    }

    #[test]
    fn build_titlebar_contains_title_text() {
        let build = sample_props().build("test");
        let titlebar_children = match &build.children[0] {
            Desc::Container { children, .. } => children,
            other => panic!("first child should be Container, got {}", desc_variant_name(other)),
        };
        assert_eq!(titlebar_children.len(), 1);
        match &titlebar_children[0] {
            Desc::Leaf { kind: LeafKind::Text { content, .. }, .. } => {
                assert_eq!(content, "Title");
            }
            other => panic!("titlebar child should be Text leaf, got {}", desc_variant_name(other)),
        }
    }

    #[test]
    fn build_content_flex_grow_holds_user_children() {
        let mut props = sample_props();
        props.content = vec![
            Desc::Leaf {
                id: Cow::Borrowed("user_child"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "inner".to_string(),
                    font_size: 12.0,
                    color: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                },
            }
        ];
        let build = props.build("test");
        match &build.children[1] {
            Desc::Container { style, decoration, children, .. } => {
                assert_eq!(style.flex_grow, 1.0);
                assert!(decoration.is_none(), "content area should be transparent");
                assert_eq!(children.len(), 1);
            }
            other => panic!("second child should be Container, got {}", desc_variant_name(other)),
        }
    }

    // ── props_eq 测试 ──

    #[test]
    fn props_eq_identical() {
        let a = sample_props();
        let b = sample_props();
        assert!(a.props_eq(&b), "相同 props 应返回 true");
    }

    #[test]
    fn props_eq_different_position() {
        let a = sample_props();
        let mut b = sample_props();
        b.x = 999.0;
        assert!(!a.props_eq(&b), "x 不同应返回 false");

        let mut c = sample_props();
        c.y = 999.0;
        assert!(!a.props_eq(&c), "y 不同应返回 false");
    }

    #[test]
    fn props_eq_different_title() {
        let a = sample_props();
        let mut b = sample_props();
        b.title = Cow::Borrowed("Different");
        assert!(!a.props_eq(&b), "title 不同应返回 false");
    }

    // ── 集成测试（与 C.1 layout + C.2 hit_test 联动）──

    #[test]
    fn hit_on_titlebar_reaches_drag_node() {
        // sample_props: x=10, y=20, w=300, h=200
        // 标题栏纵向范围：y=20 到 y=52（32px 高）
        // 点 (100, 30) 应在标题栏内
        let (tree, root) = build_tree_for_hit(sample_props());
        let chain = hit_test(&tree, root, 100.0, 30.0);
        assert!(!chain.is_empty(), "hit chain should not be empty");
        let has_drag = chain.iter().any(|id| {
            tree.get(id)
                .map(|n| n.style.gestures.contains(&Gesture::Drag))
                .unwrap_or(false)
        });
        assert!(has_drag, "hit chain should contain a node with Drag gesture");
    }

    #[test]
    fn hit_on_corner_reaches_resize_node() {
        // 外框右下角 (310, 220) = (x+w, y+h)
        let (tree, root) = build_tree_for_hit(sample_props());
        let chain = hit_test(&tree, root, 310.0, 220.0);
        assert!(!chain.is_empty(), "hit chain should not be empty");
        let has_resize = chain.iter().any(|id| {
            tree.get(id)
                .map(|n| n.style.gestures.contains(&Gesture::Resize))
                .unwrap_or(false)
        });
        assert!(has_resize, "hit chain should contain a node with Resize gesture");
    }
}
