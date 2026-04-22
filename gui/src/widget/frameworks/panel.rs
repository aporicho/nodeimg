use crate::gesture::Gesture;
use crate::renderer::{Border, Color, Rect, Shadow, TextStyle};
use crate::tree::layout::{LeafKind, Overflow, Size};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

/// Panel widget 的 props。
///
/// Widget 只负责把标题栏、内容区和交互热区组合成树；位置、尺寸、显示和层级
/// 由上层 panel runtime 维护后再写回 props。
pub struct PanelProps {
    pub title: Cow<'static, str>,
    pub rect: Rect,
    pub z_index: i32,
    pub min_size: [f32; 2],
    pub titlebar_visible: bool,
    pub draggable: bool,
    pub resizable: bool,
    pub closable: bool,
    pub content: Vec<Desc>,
}

impl Clone for PanelProps {
    fn clone(&self) -> Self {
        PanelProps {
            title: self.title.clone(),
            rect: self.rect,
            z_index: self.z_index,
            min_size: self.min_size,
            titlebar_visible: self.titlebar_visible,
            draggable: self.draggable,
            resizable: self.resizable,
            closable: self.closable,
            content: self.content.iter().map(desc_clone).collect(),
        }
    }
}

fn desc_clone(d: &Desc) -> Desc {
    d.clone()
}

impl fmt::Debug for PanelProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PanelProps")
            .field("title", &self.title)
            .field("rect", &self.rect)
            .field("z_index", &self.z_index)
            .field("min_size", &self.min_size)
            .field("titlebar_visible", &self.titlebar_visible)
            .field("draggable", &self.draggable)
            .field("resizable", &self.resizable)
            .field("closable", &self.closable)
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
        other.as_any().downcast_ref::<Self>().is_some_and(|o| {
            self.title == o.title
                && rect_eq(self.rect, o.rect)
                && self.z_index == o.z_index
                && self.min_size == o.min_size
                && self.titlebar_visible == o.titlebar_visible
                && self.draggable == o.draggable
                && self.resizable == o.resizable
                && self.closable == o.closable
        })
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let theme = cx.theme;
        let tokens = theme.components.panel;
        let visual = theme.panel_visual();
        let anatomy = Anatomy::new(id);

        // 标题栏
        let titlebar = self.titlebar_visible.then(|| {
            let mut titlebar = ui::row(anatomy.titlebar())
                .fixed_height(tokens.title_bar_height)
                .padding_symmetric(tokens.title_padding_y, tokens.title_padding_x)
                .background(visual.titlebar_background)
                .radius([tokens.radius, tokens.radius, 0.0, 0.0])
                .child(ui::leaf(
                    anatomy.title(),
                    LeafKind::Text {
                        content: self.title.to_string(),
                        style: TextStyle {
                            color: visual.title_text,
                            size: tokens.title_font_size,
                            ..theme.text_style_title_sm()
                        },
                        layout: Default::default(),
                    },
                ));
            if self.draggable {
                titlebar = titlebar.gesture(Gesture::Drag);
            }
            titlebar.build()
        });

        // 内容区（透明，事件穿透到子控件）
        let content_area = ui::column(anatomy.content())
            .flex_grow(1.0)
            .gap(tokens.content_padding)
            .padding_all(tokens.content_padding)
            .children(self.content.iter().map(desc_clone))
            .build();

        let mut root = build::column()
            .absolute_xy(self.rect.x, self.rect.y)
            .z_index(self.z_index)
            .fixed_width(self.rect.w)
            .height(if self.rect.h > 0.0 {
                Size::Fixed(self.rect.h)
            } else {
                Size::Auto
            })
            .overflow(Overflow::Hidden)
            .background(visual.frame_background)
            .border(Border {
                width: tokens.border_width,
                color: visual.frame_border,
            })
            .radius_all(tokens.radius)
            .shadow(Shadow {
                color: Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.18,
                },
                offset: [0.0, 10.0],
                blur: 24.0,
                spread: 1.0,
            })
            .children(titlebar.into_iter().chain(std::iter::once(content_area)));
        if self.resizable {
            root = root.gesture(Gesture::Resize);
        }
        root.build()
    }
}

fn rect_eq(a: Rect, b: Rect) -> bool {
    a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gesture::Gesture;
    use crate::renderer::{Color, Rect, TextStyle};
    use crate::theme::{dark_theme, Theme};
    use crate::tree::layout::{LeafKind, Position, Size};
    use crate::tree::{hit_test, layout, reconcile, NodeId, Tree};
    use crate::widget::props::WidgetBuildCx;

    fn build_cx<'a>(theme: &'a Theme) -> WidgetBuildCx<'a> {
        WidgetBuildCx {
            theme,
            force_rebuild: false,
        }
    }

    /// 标准 props：x=10, y=20, w=300, h=200，空 content。
    fn sample_props() -> PanelProps {
        PanelProps {
            title: Cow::Borrowed("Title"),
            rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 200.0,
            },
            min_size: [120.0, 80.0],
            z_index: 7,
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            content: Vec::new(),
        }
    }

    /// 集成测试 helper：把 PanelProps 装进树，走 reconcile + layout，
    /// 返回可直接 hit_test 的 Tree + root NodeId。
    fn build_tree_for_hit(props: PanelProps) -> (Tree, NodeId) {
        let theme = dark_theme();
        let desc = ui::widget(Cow::Borrowed("test_panel"), props).build();

        let mut tree = Tree::new();
        reconcile(&mut tree, desc, build_cx(&theme));

        let root = tree.root().expect("tree should have root after reconcile");

        let mut no_measure = |_text: &str, _style: &TextStyle| -> (f32, f32) { (0.0, 0.0) };
        layout(
            &mut tree,
            root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 1000.0,
            },
            &mut no_measure,
        );

        (tree, root)
    }

    fn desc_variant_name(d: &Desc) -> &'static str {
        match d {
            Desc::Container { .. } => "Container",
            Desc::Leaf { .. } => "Leaf",
            Desc::Widget(_) => "Widget",
        }
    }

    // ── 结构测试 ──

    #[test]
    fn widget_type_is_panel() {
        assert_eq!(sample_props().widget_type(), "Panel");
    }

    #[test]
    fn build_outer_is_absolute() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        match build.style.position {
            Position::Absolute(position) => {
                assert_eq!(position.inset.left, Some(10.0));
                assert_eq!(position.inset.top, Some(20.0));
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
        assert_eq!(build.style.z_index, 7);
    }

    #[test]
    fn build_outer_has_resize_gesture() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        assert_eq!(build.style.gestures, vec![Gesture::Resize]);
    }

    #[test]
    fn build_outer_overflow_is_hidden() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        assert_eq!(build.style.overflow, Overflow::Hidden);
    }

    #[test]
    fn build_children_count_is_two() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        assert_eq!(
            build.children.len(),
            2,
            "panel should have titlebar + content"
        );
    }

    #[test]
    fn build_can_hide_titlebar() {
        let theme = dark_theme();
        let mut props = sample_props();
        props.titlebar_visible = false;
        let build = props.build("test", &build_cx(&theme));

        assert_eq!(build.children.len(), 1);
        match &build.children[0] {
            Desc::Container { id, style, .. } => {
                assert_eq!(id.as_ref(), "test::content");
                assert!(style.gestures.is_empty());
            }
            other => panic!(
                "only child should be content Container, got {}",
                desc_variant_name(other)
            ),
        }
    }

    #[test]
    fn build_titlebar_has_drag_gesture() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        match &build.children[0] {
            Desc::Container { style, .. } => {
                assert_eq!(style.gestures, vec![Gesture::Drag]);
                match style.height {
                    Size::Fixed(h) => assert_eq!(h, theme.components.panel.title_bar_height),
                    other => panic!("expected Fixed titlebar height, got {:?}", other),
                }
            }
            other => panic!(
                "first child should be Container, got {}",
                desc_variant_name(other)
            ),
        }
    }

    #[test]
    fn build_titlebar_contains_title_text() {
        let theme = dark_theme();
        let build = sample_props().build("test", &build_cx(&theme));
        let titlebar_children = match &build.children[0] {
            Desc::Container { children, .. } => children,
            other => panic!(
                "first child should be Container, got {}",
                desc_variant_name(other)
            ),
        };
        assert_eq!(titlebar_children.len(), 1);
        match &titlebar_children[0] {
            Desc::Leaf {
                kind: LeafKind::Text { content, .. },
                ..
            } => {
                assert_eq!(content, "Title");
            }
            other => panic!(
                "titlebar child should be Text leaf, got {}",
                desc_variant_name(other)
            ),
        }
    }

    #[test]
    fn build_content_flex_grow_holds_user_children() {
        let theme = dark_theme();
        let mut props = sample_props();
        props.content = vec![ui::leaf(
            "user_child",
            LeafKind::Text {
                content: "inner".to_string(),
                style: TextStyle::new(
                    Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                    12.0,
                ),
                layout: Default::default(),
            },
        )
        .build()];
        let build = props.build("test", &build_cx(&theme));
        match &build.children[1] {
            Desc::Container {
                style,
                decoration,
                children,
                ..
            } => {
                assert_eq!(style.flex_grow, 1.0);
                assert_eq!(style.gap, theme.components.panel.content_padding);
                assert!(decoration.is_none(), "content area should be transparent");
                assert_eq!(children.len(), 1);
            }
            other => panic!(
                "second child should be Container, got {}",
                desc_variant_name(other)
            ),
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
        b.rect.x = 999.0;
        assert!(!a.props_eq(&b), "x 不同应返回 false");

        let mut c = sample_props();
        c.rect.y = 999.0;
        assert!(!a.props_eq(&c), "y 不同应返回 false");
    }

    #[test]
    fn props_eq_different_title() {
        let a = sample_props();
        let mut b = sample_props();
        b.title = Cow::Borrowed("Different");
        assert!(!a.props_eq(&b), "title 不同应返回 false");
    }

    #[test]
    fn props_eq_different_titlebar_visibility() {
        let a = sample_props();
        let mut b = sample_props();
        b.titlebar_visible = false;
        assert!(!a.props_eq(&b), "titlebar_visible 不同应返回 false");
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
        assert!(
            has_drag,
            "hit chain should contain a node with Drag gesture"
        );
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
        assert!(
            has_resize,
            "hit chain should contain a node with Resize gesture"
        );
    }
}
