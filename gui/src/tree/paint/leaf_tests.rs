use super::icon_svg::svg_style_from_icon;
use super::test_support::{
    assert_point_near, assert_rect_near, container_node, first_path, leaf_node, only_command,
    paint_single_leaf, paint_tree, point, rect,
};
use crate::geometry::{Affine2D, TransformSpec};
use crate::icon::IconSpec;
use crate::paint::{
    CirclePaint, Color, GridPaint, ImageFit, ImageOpacity, ImageStyle, PaintCommand, PathCommand,
    PathData, PathStyle, Stroke, TextWeight,
};
use crate::tree::layout::{
    CustomPaintFn, CustomPainter, LeafKind, TextLayout, TextOverflow, TextureHandle,
};
use crate::tree::paint_helpers::CONNECTION_WIDTH;
use crate::tree::paint_target::{CustomPaintCx, PaintTarget};
use crate::tree::Tree;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

#[test]
fn grid_leaf_records_single_grid_command() {
    let list = paint_single_leaf(
        LeafKind::Grid {
            spacing: 10.0,
            dot_color: Color::WHITE,
            dot_size: 1.5,
        },
        rect(10.0, 20.0, 100.0, 50.0),
    );

    let command = only_command(&list);
    assert_eq!(
        command.command,
        PaintCommand::Grid(GridPaint {
            rect: rect(0.0, 0.0, 100.0, 50.0),
            spacing: 10.0,
            dot_color: Color::WHITE,
            dot_size: 1.5,
        })
    );
    assert_eq!(
        command.transform.transform_point(point(0.0, 0.0)),
        point(10.0, 20.0)
    );
}

#[test]
fn line_leaf_records_local_path_and_node_transform() {
    let stroke = Stroke::new(2.0, Color::WHITE);
    let list = paint_single_leaf(
        LeafKind::Line {
            start: point(1.0, 2.0),
            end: point(3.0, 4.0),
            stroke,
        },
        rect(10.0, 20.0, 100.0, 50.0),
    );

    let command = only_command(&list);
    assert_eq!(
        command.command,
        PaintCommand::Path(crate::paint::PathPaint {
            data: PathData::line(point(1.0, 2.0), point(3.0, 4.0)),
            style: PathStyle::stroke(stroke),
        })
    );
    assert_eq!(
        command.transform.transform_point(point(0.0, 0.0)),
        point(10.0, 20.0)
    );
}

#[test]
fn icon_leaf_records_renderer_free_svg_command() {
    let spec = IconSpec::new("plus", Color::WHITE);
    let list = paint_single_leaf(
        LeafKind::Icon { spec: spec.clone() },
        rect(10.0, 20.0, 16.0, 16.0),
    );

    let command = only_command(&list);
    assert!(matches!(
        &command.command,
        PaintCommand::Svg(svg)
            if svg.rect == rect(0.0, 0.0, 16.0, 16.0)
                && svg.source.id == "plus"
                && svg.style == svg_style_from_icon(spec.style)
    ));
}

#[test]
fn image_leaf_records_local_rect_and_style() {
    let texture = TextureHandle(42);
    let style = ImageStyle::default()
        .with_fit(ImageFit::Contain)
        .with_opacity(ImageOpacity::new(0.5));
    let list = paint_single_leaf(
        LeafKind::Image { texture, style },
        rect(10.0, 20.0, 30.0, 40.0),
    );

    let command = only_command(&list);
    assert!(matches!(
        command.command,
        PaintCommand::Image(image)
            if image.rect == rect(0.0, 0.0, 30.0, 40.0)
                && image.texture == texture
                && image.style == style
    ));
}

#[test]
fn text_leaf_records_visible_and_clipped_local_text_commands() {
    let visible = paint_single_leaf(
        LeafKind::Text {
            content: "hello".to_string(),
            style: crate::paint::TextStyle::new(Color::WHITE, 12.0).with_weight(TextWeight::Medium),
            layout: TextLayout::default(),
        },
        rect(10.0, 20.0, 100.0, 20.0),
    );
    assert!(matches!(
        only_command(&visible).command,
        PaintCommand::Text(crate::paint::TextPaint { bounds: None, .. })
    ));

    let clipped = paint_single_leaf(
        LeafKind::Text {
            content: "hello".to_string(),
            style: crate::paint::TextStyle::new(Color::WHITE, 12.0),
            layout: TextLayout {
                overflow: TextOverflow::Clip,
                ..TextLayout::default()
            },
        },
        rect(10.0, 20.0, 100.0, 20.0),
    );
    assert!(matches!(
        only_command(&clipped).command,
        PaintCommand::Text(crate::paint::TextPaint {
            bounds: Some(bounds),
            ..
        }) if bounds == rect(0.0, 0.0, 100.0, 20.0)
    ));
}

#[test]
fn connection_leaf_records_path_from_port_centers_in_connection_local_space() {
    let mut tree = Tree::new();
    let from = tree.insert(leaf_node(
        "from_port",
        LeafKind::Circle {
            radius: 4.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(10.0, 20.0, 8.0, 8.0),
    ));
    let to = tree.insert(leaf_node(
        "to_port",
        LeafKind::Circle {
            radius: 4.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(50.0, 40.0, 8.0, 8.0),
    ));
    let connection = tree.insert(leaf_node(
        "connection",
        LeafKind::Connection {
            from_port: Cow::Borrowed("from_port"),
            to_port: Cow::Borrowed("to_port"),
        },
        rect(0.0, 0.0, 0.0, 0.0),
    ));
    let root = tree.insert(container_node(
        "root",
        rect(0.0, 0.0, 100.0, 100.0),
        vec![from, to, connection],
    ));
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let path = first_path(&list);

    assert_eq!(
        path.data.commands,
        vec![
            PathCommand::MoveTo(point(14.0, 24.0)),
            PathCommand::CubicTo(point(34.0, 24.0), point(34.0, 44.0), point(54.0, 44.0)),
        ]
    );
    assert_eq!(path.style.stroke.unwrap().width, CONNECTION_WIDTH);
}

#[test]
fn connection_leaf_resolves_port_centers_under_affine_transform() {
    let mut tree = Tree::new();
    let from = tree.insert(leaf_node(
        "from_port",
        LeafKind::Circle {
            radius: 2.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(10.0, 0.0, 4.0, 4.0),
    ));
    let mut port_group = container_node("port_group", rect(20.0, 30.0, 40.0, 40.0), vec![from]);
    port_group.style.transform = Some(TransformSpec::translate_scale_rotate(
        [0.0, 0.0],
        2.0,
        std::f32::consts::FRAC_PI_2,
    ));
    let port_group = tree.insert(port_group);
    let to = tree.insert(leaf_node(
        "to_port",
        LeafKind::Circle {
            radius: 2.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(120.0, 140.0, 4.0, 4.0),
    ));
    let connection = tree.insert(leaf_node(
        "connection",
        LeafKind::Connection {
            from_port: Cow::Borrowed("from_port"),
            to_port: Cow::Borrowed("to_port"),
        },
        rect(10.0, 20.0, 0.0, 0.0),
    ));
    let root = tree.insert(container_node(
        "root",
        rect(0.0, 0.0, 200.0, 200.0),
        vec![port_group, to, connection],
    ));
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let path = first_path(&list);

    assert_eq!(
        path.data.commands,
        vec![
            PathCommand::MoveTo(point(6.0, 34.0)),
            PathCommand::CubicTo(point(59.0, 34.0), point(59.0, 122.0), point(112.0, 122.0)),
        ]
    );
}

#[test]
fn pending_connection_resolves_cursor_canvas_under_affine_space() {
    let mut tree = Tree::new();
    let from = tree.insert(leaf_node(
        "from_port",
        LeafKind::Circle {
            radius: 2.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(10.0, 20.0, 4.0, 4.0),
    ));
    let pending = tree.insert(leaf_node(
        "pending",
        LeafKind::PendingConnection {
            from_port: Cow::Borrowed("from_port"),
            cursor_canvas: point(50.0, 70.0),
        },
        rect(0.0, 0.0, 0.0, 0.0),
    ));
    let mut canvas_root = container_node(
        "canvas_root",
        rect(0.0, 0.0, 200.0, 200.0),
        vec![from, pending],
    );
    canvas_root.style.transform = Some(TransformSpec::translate_scale([100.0, 50.0], 2.0));
    let canvas_root = tree.insert(canvas_root);
    let root = tree.insert(container_node(
        "root",
        rect(0.0, 0.0, 200.0, 200.0),
        vec![canvas_root],
    ));
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let path = first_path(&list);

    assert_eq!(
        path.data.commands,
        vec![
            PathCommand::MoveTo(point(12.0, 22.0)),
            PathCommand::CubicTo(point(31.0, 22.0), point(31.0, 70.0), point(50.0, 70.0)),
        ]
    );
}

#[derive(Debug)]
struct TestCustomPainter;

impl CustomPainter for TestCustomPainter {
    fn paint(&self, target: &mut dyn PaintTarget, cx: CustomPaintCx) {
        assert_eq!(cx.local_rect, rect(0.0, 0.0, 8.0, 10.0));
        target.draw_circle(
            crate::geometry::Point {
                x: cx.local_rect.w * 0.5,
                y: cx.local_rect.h * 0.5,
            },
            3.0,
            Color::WHITE,
        );
    }
}

#[test]
fn custom_paint_leaf_invokes_painter_with_local_rect() {
    let list = paint_single_leaf(
        LeafKind::CustomPaint(CustomPaintFn(Arc::new(TestCustomPainter))),
        rect(10.0, 20.0, 8.0, 10.0),
    );

    let command = only_command(&list);
    assert_eq!(
        command.command,
        PaintCommand::Circle(CirclePaint {
            center: point(4.0, 5.0),
            radius: 3.0,
            fill: Some(Color::WHITE),
            stroke: None,
        })
    );
    assert_eq!(
        command.transform.transform_point(point(0.0, 0.0)),
        point(10.0, 20.0)
    );
}

#[derive(Debug)]
struct CapturingCustomPainter {
    captured: Arc<Mutex<Option<CustomPaintCx>>>,
}

impl CustomPainter for CapturingCustomPainter {
    fn paint(&self, target: &mut dyn PaintTarget, cx: CustomPaintCx) {
        *self.captured.lock().expect("capture lock") = Some(cx);
        target.draw_circle(
            crate::geometry::Point {
                x: cx.local_rect.w * 0.5,
                y: cx.local_rect.h * 0.5,
            },
            3.0,
            Color::WHITE,
        );
    }
}

#[test]
fn custom_paint_cx_reports_affine_transform_and_screen_bounds() {
    let captured = Arc::new(Mutex::new(None));
    let mut tree = Tree::new();
    let custom = tree.insert(leaf_node(
        "custom",
        LeafKind::CustomPaint(CustomPaintFn(Arc::new(CapturingCustomPainter {
            captured: captured.clone(),
        }))),
        rect(3.0, 4.0, 8.0, 10.0),
    ));
    let parent_transform =
        TransformSpec::translate_scale_rotate([5.0, 7.0], 2.0, std::f32::consts::FRAC_PI_2);
    let mut parent = container_node("parent", rect(20.0, 30.0, 40.0, 40.0), vec![custom]);
    parent.style.transform = Some(parent_transform);
    let parent = tree.insert(parent);
    let root = tree.insert(container_node(
        "root",
        rect(0.0, 0.0, 100.0, 100.0),
        vec![parent],
    ));
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let cx = captured
        .lock()
        .expect("capture lock")
        .expect("custom paint context");
    let expected_parent = Affine2D::compose(
        Affine2D::translation(20.0, 30.0),
        parent_transform.to_affine(rect(0.0, 0.0, 40.0, 40.0)),
    );
    let expected_transform = Affine2D::compose(expected_parent, Affine2D::translation(3.0, 4.0));
    let expected_bounds = expected_transform.transformed_bounds(rect(0.0, 0.0, 8.0, 10.0));

    assert_eq!(cx.local_rect, rect(0.0, 0.0, 8.0, 10.0));
    assert_point_near(
        cx.transform.transform_point(point(4.0, 5.0)),
        expected_transform.transform_point(point(4.0, 5.0)),
    );
    assert_rect_near(cx.screen_bounds, expected_bounds);
    assert!(matches!(
        only_command(&list).command,
        PaintCommand::Circle(CirclePaint { .. })
    ));
}
