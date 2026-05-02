mod container;
mod decoration;
mod leaf;
mod style;
mod widget;

pub use container::{column, container, row, ContainerBuilder};
pub use decoration::DecorationBuilder;
pub use leaf::{icon, leaf, text, text_with_layout, LeafBuilder};
pub use style::StyleBuilder;
pub use widget::widget;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, TextStyle};
    use crate::tree::layout::{
        Align, Direction, LeafKind, Overflow, Position, Size, TextAlign, TextLayout, TextOverflow,
    };
    use crate::tree::Desc;
    use crate::widget::atoms::dot::DotProps;

    #[test]
    fn row_and_column_set_flex_direction() {
        let Desc::Container { style, .. } = row("row").build() else {
            panic!("expected row container");
        };
        assert_eq!(style.direction, Direction::Row);

        let Desc::Container { style, .. } = column("column").build() else {
            panic!("expected column container");
        };
        assert_eq!(style.direction, Direction::Column);
    }

    #[test]
    fn style_methods_write_box_style() {
        let Desc::Container { style, .. } = row("style")
            .fixed_width(20.0)
            .fixed_height(10.0)
            .min_width(2.0)
            .max_width(40.0)
            .min_height(3.0)
            .max_height(30.0)
            .padding_all(4.0)
            .margin_all(5.0)
            .gap(6.0)
            .align_items(Align::Center)
            .align_self(Align::End)
            .flex_grow(1.0)
            .flex_shrink(1.0)
            .overflow(Overflow::Hidden)
            .absolute_xy(7.0, 8.0)
            .z_index(9)
            .hittable(true)
            .draggable(true)
            .resizable(true)
            .resize_edge_threshold(14.0)
            .build()
        else {
            panic!("expected container");
        };

        assert_eq!(style.width, Size::Fixed(20.0));
        assert_eq!(style.height, Size::Fixed(10.0));
        assert_eq!(style.min_width, 2.0);
        assert_eq!(style.max_width, 40.0);
        assert_eq!(style.min_height, 3.0);
        assert_eq!(style.max_height, 30.0);
        assert_eq!(style.padding.top, 4.0);
        assert_eq!(style.margin.top, 5.0);
        assert_eq!(style.gap, 6.0);
        assert_eq!(style.align_items, Align::Center);
        assert_eq!(style.align_self, Some(Align::End));
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.flex_shrink, 1.0);
        assert_eq!(style.overflow, Overflow::Hidden);
        assert_eq!(style.position, Position::absolute_xy(7.0, 8.0));
        assert_eq!(style.z_index, 9);
        assert_eq!(style.hittable, Some(true));
        assert!(style.draggable);
        assert!(style.resizable);
        assert_eq!(style.resize_edge_threshold, 14.0);
    }

    #[test]
    fn text_builds_text_leaf_with_layout() {
        let layout = TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Center,
        };
        let Desc::Leaf { kind, style, .. } =
            text_with_layout("label", "Hello", TextStyle::new(Color::WHITE, 12.0), layout)
                .fill_width()
                .build()
        else {
            panic!("expected text leaf");
        };

        assert_eq!(style.width, Size::Fill);
        let LeafKind::Text {
            content,
            layout: actual_layout,
            ..
        } = kind
        else {
            panic!("expected text kind");
        };
        assert_eq!(content, "Hello");
        assert_eq!(actual_layout, layout);
    }

    #[test]
    fn children_accept_builders_and_widget_desc() {
        let dot = widget(
            "dot",
            DotProps {
                diameter: 4.0,
                fill: Color::WHITE,
                border: None,
                hittable: false,
                gestures: Vec::new(),
            },
        );

        let Desc::Container { children, .. } = column("root")
            .child(row("row_child"))
            .child(text(
                "text_child",
                "Text",
                TextStyle::new(Color::WHITE, 12.0),
            ))
            .child(dot)
            .build()
        else {
            panic!("expected container");
        };

        assert_eq!(children.len(), 3);
        assert!(matches!(children[0], Desc::Container { .. }));
        assert!(matches!(children[1], Desc::Leaf { .. }));
        assert!(matches!(children[2], Desc::Widget(_)));
    }

    #[test]
    fn widget_builder_keeps_external_children() {
        let desc = widget(
            "wrapper",
            DotProps {
                diameter: 4.0,
                fill: Color::WHITE,
                border: None,
                hittable: false,
                gestures: Vec::new(),
            },
        )
        .child(row("external"))
        .build();

        let Desc::Widget(widget) = desc else {
            panic!("expected widget");
        };
        assert_eq!(widget.children().len(), 1);
    }
}
