use super::{canvas_node_stable_id, CanvasNodeLayout};
use crate::gesture::Gesture;
use crate::renderer::Border;
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind, Position, Size};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct CanvasNodeView {
    pub owner_id: String,
    pub title: String,
    pub subtitle: String,
    pub layout: CanvasNodeLayout,
}

pub fn node_card(view: &CanvasNodeView, theme: &Theme) -> Desc {
    let card_id = canvas_node_stable_id(&view.owner_id);
    let title_id = format!("{card_id}::title");
    let subtitle_id = format!("{card_id}::subtitle");

    Desc::Container {
        id: Cow::Owned(card_id),
        style: BoxStyle {
            position: Position::Absolute {
                x: view.layout.rect.x,
                y: view.layout.rect.y,
            },
            width: Size::Fixed(view.layout.rect.w),
            height: Size::Fixed(if view.layout.collapsed {
                48.0
            } else {
                view.layout.rect.h
            }),
            padding: Edges::all(10.0),
            gap: 4.0,
            direction: Direction::Column,
            hittable: Some(true),
            gestures: vec![Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.surface),
            border: Some(Border {
                width: 1.0,
                color: theme.colors.border,
            }),
            radius: [theme.radii.md; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(title_id),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: view.title.clone(),
                    style: theme.text_style_title_sm(),
                },
            },
            Desc::Leaf {
                id: Cow::Owned(subtitle_id),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: view.subtitle.clone(),
                    style: theme.text_style_label_sm(),
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::theme::light_theme;

    #[test]
    fn node_card_uses_canvas_node_stable_id() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        assert_eq!(
            node_card(&view, &light_theme()).id(),
            "canvas_node::engine_node::7"
        );
    }

    #[test]
    fn node_card_declares_drag_gesture() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        let Desc::Container { style, .. } = node_card(&view, &light_theme()) else {
            panic!("node card should build a container");
        };
        assert!(style.gestures.contains(&Gesture::Drag));
    }
}
