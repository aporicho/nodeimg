use super::connection_endpoint::node_screen_center;
use super::layout::{LeafKind, Overflow};
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{connection_path, CONNECTION_WIDTH};
use super::paint_space::{NodePaintSpace, PaintSpace};
use super::paint_target::{CustomPaintCx, PaintTarget};
use super::text_layout::resolve_text_paint;
use super::tree::Tree;
use super::{RepaintBoundaryId, Revision};
use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::{Affine2D, Point, Rect};
use crate::icon::{IconFit, IconPaintOverride, IconStrokeWidth, IconStyle};
use crate::interaction::InteractionState;
use crate::paint::{
    BoundaryPaintRecorder, CirclePaint, ClipShape, Color, DisplayList, FragmentChildRef, GridPaint,
    LayerPaint, PaintBuildError, PaintCommand, PaintFragment, PathData, PathStyle,
    RecordingPaintTarget, RectStyle, Stroke, SvgFit, SvgPaintOverride, SvgSourceKey,
    SvgStrokeWidth, SvgStyle, TextStyle,
};
use crate::theme::Theme;
use crate::widget::painters::{
    paint_text_leaf_override as paint_widget_text_leaf_override,
    widget_visual_override as paint_widget_visual_override,
};
use crate::widget::state::TextBoxStore;

pub(crate) struct PaintCx<'a> {
    pub(crate) interaction: Option<&'a InteractionState>,
    pub(crate) text_boxes: Option<&'a TextBoxStore>,
    pub(crate) animations: Option<&'a AnimationStore>,
    pub(crate) theme: &'a Theme,
}

pub(crate) fn build_paint_fragment(
    tree: &Tree,
    boundary: RepaintBoundaryId,
    cx: PaintCx<'_>,
    measure_text: impl FnMut(&str, &TextStyle) -> (f32, f32),
) -> Result<PaintFragment, PaintBuildError> {
    let Some(node) = tree.get(boundary.0) else {
        return Ok(empty_fragment(boundary));
    };
    let origin = Point {
        x: node.rect.x,
        y: node.rect.y,
    };
    let boundary_to_screen = Affine2D::translation(node.rect.x, node.rect.y);
    let local_bounds = Rect {
        x: 0.0,
        y: 0.0,
        w: node.rect.w,
        h: node.rect.h,
    };
    let revision = node.paint_meta.fragment_revision;
    let mut recorder = BoundaryPaintRecorder::with_measure(measure_text);
    recorder
        .target()
        .push_transform(Affine2D::translation(-origin.x, -origin.y));
    let mut child_boundaries = Vec::new();
    let mut traversal = PaintTraversal::Boundary {
        root: boundary.0,
        boundary_to_screen,
        child_boundaries: &mut child_boundaries,
    };
    paint_to_target(
        tree,
        boundary.0,
        recorder.target(),
        cx.interaction,
        cx.text_boxes,
        cx.animations,
        cx.theme,
        &mut traversal,
    );
    recorder.target().pop_transform();
    recorder.finish(boundary, revision, local_bounds, child_boundaries)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_to_target(
    tree: &Tree,
    root: NodeId,
    target: &mut dyn PaintTarget,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    traversal: &mut PaintTraversal<'_>,
) {
    paint_node(
        tree,
        root,
        target,
        PaintSpace::root(),
        interaction,
        text_boxes,
        animations,
        theme,
        None,
        traversal,
    );
}

pub(crate) enum PaintTraversal<'a> {
    Full,
    Boundary {
        root: NodeId,
        boundary_to_screen: Affine2D,
        child_boundaries: &'a mut Vec<FragmentChildRef>,
    },
}

fn empty_fragment(boundary: RepaintBoundaryId) -> PaintFragment {
    PaintFragment {
        boundary,
        revision: Revision::ZERO,
        local_bounds: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        clips: Vec::new(),
        commands: Vec::new(),
        child_boundaries: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    current_space: PaintSpace,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
    traversal: &mut PaintTraversal<'_>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };
    tree.record_paint_node_visited();

    if let PaintTraversal::Boundary {
        root,
        boundary_to_screen,
        child_boundaries,
        ..
    } = traversal
    {
        if node_id != *root && node.paint_meta.boundary.is_some() {
            let node_space = current_space.node_space(node.rect, node.style.transform);
            let local_transform = boundary_to_screen
                .inverse()
                .map(|inverse| Affine2D::compose(inverse, node_space.local_to_screen))
                .unwrap_or_else(|| Affine2D::translation(node.rect.x, node.rect.y));
            child_boundaries.push(FragmentChildRef {
                boundary: RepaintBoundaryId(node_id),
                local_transform,
                clip_stack: Vec::new(),
                z_index: node.style.z_index,
            });
            return;
        }
    }

    if let Some(visual) = animations.and_then(|store| store.visual_for(node.id.as_ref())) {
        if visual.opacity <= 0.0 {
            return;
        }

        let layer = {
            let mut layer_target =
                RecordingPaintTarget::with_measure(|text, style| target.measure_text(text, style));
            paint_node_inner(
                tree,
                node_id,
                &mut layer_target,
                current_space,
                interaction,
                text_boxes,
                animations,
                theme,
                inherited_text_color,
                traversal,
            );
            match layer_target.display_list() {
                Ok(layer) => layer,
                Err(err) => {
                    tracing::warn!("failed to build animated node layer: {:?}", err);
                    return;
                }
            }
        };

        let pushed_transform = visual_affine(node.rect, visual);
        if let Some(transform) = pushed_transform {
            target.push_transform(transform);
        }
        target.draw(PaintCommand::Layer(LayerPaint::new(
            node.rect,
            visual.opacity,
            layer,
        )));
        if pushed_transform.is_some() {
            target.pop_transform();
        }
        return;
    }

    paint_node_inner(
        tree,
        node_id,
        target,
        current_space,
        interaction,
        text_boxes,
        animations,
        theme,
        inherited_text_color,
        traversal,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_node_inner(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    current_space: PaintSpace,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
    traversal: &mut PaintTraversal<'_>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };

    let node_space = current_space.node_space(node.rect, node.style.transform);
    let local_rect = node_space.local_rect;
    let transform = node.style.transform;
    let should_clip_children = matches!(node.style.overflow, Overflow::Hidden | Overflow::Scroll);
    let clip_radius = node
        .decoration
        .as_ref()
        .map(|decoration| decoration.radius)
        .unwrap_or([0.0; 4]);
    let children = tree.children_in_paint_order_cached(node_id, &node.children);
    let mut child_text_color = inherited_text_color;

    target.push_transform(Affine2D::translation(node.rect.x, node.rect.y));

    if let Some((style, text_color)) =
        paint_widget_visual_override(tree, node_id, interaction, theme)
    {
        target.draw_rect(local_rect, style);
        child_text_color = Some(text_color);
    } else if let Some(decoration) = &node.decoration {
        target.draw_rect(
            local_rect,
            RectStyle {
                color: decoration.background.unwrap_or(Color::TRANSPARENT),
                border: decoration.border,
                radius: decoration.radius,
                shadow: decoration.shadow,
            },
        );
    }

    if should_clip_children {
        target.push_clip(ClipShape::RoundedRect {
            rect: local_rect,
            radius: clip_radius,
        });
    }

    if let NodeKind::Leaf(leaf) = &node.kind {
        paint_leaf(
            tree,
            node_id,
            leaf,
            target,
            node.rect,
            node_space,
            current_space,
            interaction,
            text_boxes,
            theme,
            child_text_color,
        );
    }

    if let Some(transform) = transform {
        target.push_transform(transform.to_affine(local_rect));
        let child_space = node_space.child_space();
        for child_id in children {
            paint_node(
                tree,
                child_id,
                target,
                child_space,
                interaction,
                text_boxes,
                animations,
                theme,
                child_text_color,
                traversal,
            );
        }
        target.pop_transform();
        if should_clip_children {
            target.pop_clip();
        }
        target.pop_transform();
    } else {
        target.pop_transform();
        for child_id in children {
            paint_node(
                tree,
                child_id,
                target,
                current_space,
                interaction,
                text_boxes,
                animations,
                theme,
                child_text_color,
                traversal,
            );
        }
        if should_clip_children {
            target.pop_clip();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_leaf(
    tree: &Tree,
    node_id: NodeId,
    leaf: &LeafKind,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    node_space: NodePaintSpace,
    current_space: PaintSpace,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
) {
    let local_rect = node_space.local_rect;

    match leaf {
        LeafKind::Text {
            content,
            style,
            layout,
        } => {
            let text_style = with_inherited_text_color(*style, inherited_text_color);
            if !paint_widget_text_leaf_override(
                tree,
                node_id,
                target,
                node_rect,
                interaction,
                text_boxes,
                theme,
                content,
                &text_style,
            ) {
                let resolved =
                    resolve_text_paint(content, &text_style, *layout, local_rect, |text, style| {
                        target.measure_text(text, style)
                    });
                if let Some(bounds) = resolved.bounds {
                    target.draw_text_clipped(resolved.pos, &resolved.content, text_style, bounds);
                } else {
                    target.draw_text(resolved.pos, &resolved.content, text_style);
                }
            }
        }
        LeafKind::Grid {
            spacing,
            dot_color,
            dot_size,
        } => {
            target.draw_grid(GridPaint {
                rect: local_rect,
                spacing: *spacing,
                dot_color: *dot_color,
                dot_size: *dot_size,
            });
        }
        LeafKind::Image { texture, style } => {
            target.draw_image(local_rect, *texture, *style);
        }
        LeafKind::Icon { spec } => {
            target.draw_svg(
                local_rect,
                SvgSourceKey::new(spec.id.as_str()),
                svg_style_from_icon(spec.style),
            );
        }
        LeafKind::Circle {
            radius,
            fill,
            stroke,
        } => {
            target.draw_circle_paint(CirclePaint {
                center: Point {
                    x: local_rect.w * 0.5,
                    y: local_rect.h * 0.5,
                },
                radius: *radius,
                fill: *fill,
                stroke: stroke.map(|border| Stroke::new(border.width, border.color)),
            });
        }
        LeafKind::Connection { from_port, to_port } => {
            let Some(from_screen) = node_screen_center(tree, from_port.as_ref()) else {
                return;
            };
            let Some(to_screen) = node_screen_center(tree, to_port.as_ref()) else {
                return;
            };
            let Some(inverse) = node_space.local_to_screen.inverse() else {
                return;
            };
            let from_local = inverse.transform_point(from_screen);
            let to_local = inverse.transform_point(to_screen);
            target.draw_path(
                connection_path(from_local, to_local),
                PathStyle::stroke(Stroke::new(CONNECTION_WIDTH, theme.colors.connection)),
            );
        }
        LeafKind::PendingConnection {
            from_port,
            cursor_canvas,
        } => {
            let Some(from_screen) = node_screen_center(tree, from_port.as_ref()) else {
                return;
            };
            let Some(inverse) = node_space.local_to_screen.inverse() else {
                return;
            };
            let to_screen = current_space.to_screen.transform_point(*cursor_canvas);
            target.draw_path(
                connection_path(
                    inverse.transform_point(from_screen),
                    inverse.transform_point(to_screen),
                ),
                PathStyle::stroke(Stroke::new(CONNECTION_WIDTH, theme.colors.accent)),
            );
        }
        LeafKind::Line { start, end, stroke } => {
            target.draw_path(PathData::line(*start, *end), PathStyle::stroke(*stroke));
        }
        LeafKind::Curve { points, stroke } => {
            target.draw_path(PathData::cubic(*points), PathStyle::stroke(*stroke));
        }
        LeafKind::Path { data, style } => {
            target.draw_path(data.clone(), *style);
        }
        LeafKind::CustomPaint(custom) => {
            custom.0.paint(
                target,
                CustomPaintCx {
                    local_rect,
                    transform: node_space.local_to_screen,
                    screen_bounds: node_space.screen_bounds(),
                },
            );
        }
    }
}

fn with_inherited_text_color(mut style: TextStyle, inherited: Option<Color>) -> TextStyle {
    if let Some(color) = inherited {
        style.color = color;
    }
    style
}

fn svg_style_from_icon(style: IconStyle) -> SvgStyle {
    SvgStyle {
        color: style.color,
        fill: svg_paint_override_from_icon(style.fill),
        stroke: svg_paint_override_from_icon(style.stroke),
        stroke_width: svg_stroke_width_from_icon(style.stroke_width),
        opacity: style.opacity.get(),
        fit: svg_fit_from_icon(style.fit),
    }
}

fn svg_paint_override_from_icon(override_paint: IconPaintOverride) -> SvgPaintOverride {
    match override_paint {
        IconPaintOverride::Preserve => SvgPaintOverride::Preserve,
        IconPaintOverride::ReplaceCurrent(color) => SvgPaintOverride::ReplaceCurrent(color),
        IconPaintOverride::Force(color) => SvgPaintOverride::Force(color),
        IconPaintOverride::None => SvgPaintOverride::None,
    }
}

fn svg_stroke_width_from_icon(stroke_width: IconStrokeWidth) -> SvgStrokeWidth {
    match stroke_width {
        IconStrokeWidth::Preserve => SvgStrokeWidth::Preserve,
        IconStrokeWidth::SvgUnits(width) => SvgStrokeWidth::SvgUnits(width),
    }
}

fn svg_fit_from_icon(fit: IconFit) -> SvgFit {
    match fit {
        IconFit::Stretch => SvgFit::Stretch,
        IconFit::Contain => SvgFit::Contain,
    }
}

#[allow(dead_code)]
fn build_display_list_for_test(
    tree: &Tree,
    root: NodeId,
    theme: &Theme,
) -> Result<DisplayList, PaintBuildError> {
    let mut target = RecordingPaintTarget::new();
    let mut traversal = PaintTraversal::Full;
    paint_to_target(
        tree,
        root,
        &mut target,
        None,
        None,
        None,
        theme,
        &mut traversal,
    );
    target.display_list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationProps, AnimationStore, Ease};
    use crate::geometry::TransformSpec;
    use crate::icon::IconSpec;
    use crate::paint::{
        Border, ImageFit, ImageOpacity, ImageStyle, PaintCommand, PathCommand, RectPaint,
        ResolvedClip, ResolvedPaintCommand, TextWeight,
    };
    use crate::tree::layout::{
        BoxStyle, CustomPaintFn, CustomPainter, LeafKind, Overflow, TextLayout, TextOverflow,
        TextureHandle,
    };
    use crate::tree::node::{NodeLocalRuntime, TreeNode};
    use crate::tree::RepaintBoundaryReason;
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    const EPS: f32 = 1e-5;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    fn assert_rect_near(actual: Rect, expected: Rect) {
        assert!(
            (actual.x - expected.x).abs() < EPS
                && (actual.y - expected.y).abs() < EPS
                && (actual.w - expected.w).abs() < EPS
                && (actual.h - expected.h).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    fn leaf_node(id: &'static str, kind: LeafKind, rect: Rect) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Leaf(kind),
            rect,
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn container_node(id: &'static str, rect: Rect, children: Vec<NodeId>) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Container,
            rect,
            children,
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn paint_tree(tree: &Tree, root: NodeId) -> DisplayList {
        build_display_list_for_test(tree, root, &Theme::default()).unwrap()
    }

    fn paint_tree_with_animations(
        tree: &Tree,
        root: NodeId,
        animations: &AnimationStore,
    ) -> DisplayList {
        let mut target = RecordingPaintTarget::new();
        let mut traversal = PaintTraversal::Full;
        paint_to_target(
            tree,
            root,
            &mut target,
            None,
            None,
            Some(animations),
            &Theme::default(),
            &mut traversal,
        );
        target.display_list().unwrap()
    }

    fn paint_single_leaf(kind: LeafKind, rect: Rect) -> DisplayList {
        let mut tree = Tree::new();
        let root = tree.insert(leaf_node("leaf", kind, rect));
        tree.set_root(root);
        paint_tree(&tree, root)
    }

    fn paint_cx<'a>(theme: &'a Theme) -> PaintCx<'a> {
        PaintCx {
            interaction: None,
            text_boxes: None,
            animations: None,
            theme,
        }
    }

    fn only_command(list: &DisplayList) -> &ResolvedPaintCommand {
        assert_eq!(list.commands.len(), 1);
        &list.commands[0]
    }

    fn only_clip(list: &DisplayList) -> &ResolvedClip {
        assert_eq!(list.clips.len(), 1);
        &list.clips[0]
    }

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

    fn first_path(list: &DisplayList) -> &crate::paint::PathPaint {
        list.commands
            .iter()
            .find_map(|command| match &command.command {
                PaintCommand::Path(path) => Some(path),
                _ => None,
            })
            .expect("expected path command")
    }

    #[test]
    fn fragment_records_boundary_local_coordinates() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let mut tree = Tree::new();
        let root = tree.insert(leaf_node(
            "leaf",
            LeafKind::Line {
                start: point(1.0, 2.0),
                end: point(3.0, 4.0),
                stroke,
            },
            rect(10.0, 20.0, 100.0, 50.0),
        ));
        tree.set_root(root);

        let fragment = build_paint_fragment(
            &tree,
            RepaintBoundaryId(root),
            paint_cx(&Theme::default()),
            |_, _| (0.0, 0.0),
        )
        .expect("fragment");

        assert_eq!(fragment.commands.len(), 1);
        assert_eq!(
            fragment.commands[0]
                .transform
                .transform_point(point(0.0, 0.0)),
            point(0.0, 0.0)
        );
    }

    #[test]
    fn fragment_records_child_repaint_boundary_ref_without_painting_child() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Line {
                start: point(1.0, 2.0),
                end: point(3.0, 4.0),
                stroke,
            },
            rect(20.0, 30.0, 20.0, 20.0),
        ));
        tree.set_repaint_boundary(child, RepaintBoundaryReason::CanvasNodeCard);
        let root = tree.insert(container_node(
            "root",
            rect(10.0, 10.0, 100.0, 80.0),
            vec![child],
        ));
        tree.set_root(root);

        let fragment = build_paint_fragment(
            &tree,
            RepaintBoundaryId(root),
            paint_cx(&Theme::default()),
            |_, _| (0.0, 0.0),
        )
        .expect("fragment");

        assert!(fragment.commands.is_empty());
        assert_eq!(fragment.child_boundaries.len(), 1);
        assert_eq!(
            fragment.child_boundaries[0].boundary,
            RepaintBoundaryId(child)
        );
        assert_eq!(
            fragment.child_boundaries[0]
                .local_transform
                .transform_point(point(0.0, 0.0)),
            point(10.0, 20.0)
        );
    }

    #[test]
    fn fragment_child_boundary_ref_includes_ancestor_transform() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Line {
                start: point(1.0, 2.0),
                end: point(3.0, 4.0),
                stroke,
            },
            rect(20.0, 30.0, 20.0, 20.0),
        ));
        tree.set_repaint_boundary(child, RepaintBoundaryReason::CanvasNodeCard);
        let mut canvas = container_node("canvas", rect(10.0, 10.0, 100.0, 80.0), vec![child]);
        canvas.style.transform = Some(TransformSpec::translate_scale([100.0, 50.0], 2.0));
        let canvas = tree.insert(canvas);
        tree.set_root(canvas);

        let fragment = build_paint_fragment(
            &tree,
            RepaintBoundaryId(canvas),
            paint_cx(&Theme::default()),
            |_, _| (0.0, 0.0),
        )
        .expect("fragment");

        assert_eq!(fragment.child_boundaries.len(), 1);
        assert_eq!(
            fragment.child_boundaries[0]
                .local_transform
                .transform_point(point(0.0, 0.0)),
            point(140.0, 110.0)
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
                style: TextStyle::new(Color::WHITE, 12.0).with_weight(TextWeight::Medium),
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
                style: TextStyle::new(Color::WHITE, 12.0),
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
        let path = list
            .commands
            .iter()
            .find_map(|command| match &command.command {
                PaintCommand::Path(path) => Some(path),
                _ => None,
            })
            .expect("connection should record a path command");

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

    #[test]
    fn animated_node_records_layer_with_opacity_and_transform() {
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Circle {
                radius: 4.0,
                fill: Some(Color::WHITE),
                stroke: None,
            },
            rect(10.0, 20.0, 8.0, 8.0),
        ));
        let root = tree.insert(container_node(
            "root",
            rect(0.0, 0.0, 100.0, 100.0),
            vec![child],
        ));
        tree.set_root(root);

        let mut animations = AnimationStore::new();
        let now = Instant::now();
        animations
            .animate("child")
            .to(AnimationProps::new().opacity(0.5).translate([12.0, 0.0]))
            .duration_ms(100)
            .ease(Ease::Linear)
            .play_at(now);
        animations.tick(now + Duration::from_millis(100));

        let list = paint_tree_with_animations(&tree, root, &animations);
        let command = only_command(&list);

        match &command.command {
            PaintCommand::Layer(layer) => {
                assert_eq!(layer.opacity, 0.5);
                assert_eq!(layer.content.commands.len(), 1);
            }
            other => panic!("expected animated layer, got {other:?}"),
        }
        assert_eq!(
            command.transform.transform_point(point(10.0, 20.0)),
            point(22.0, 20.0)
        );
    }

    #[test]
    fn overflow_hidden_records_local_clip_around_children() {
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Line {
                start: point(0.0, 0.0),
                end: point(10.0, 0.0),
                stroke: Stroke::new(1.0, Color::WHITE),
            },
            rect(0.0, 0.0, 10.0, 10.0),
        ));
        let mut root_node = container_node("root", rect(1.0, 2.0, 30.0, 40.0), vec![child]);
        root_node.style.overflow = Overflow::Hidden;
        let root = tree.insert(root_node);
        tree.set_root(root);

        let list = paint_tree(&tree, root);
        let clip = only_clip(&list);

        assert!(matches!(
            clip.shape,
            ClipShape::RoundedRect {
                rect: clip_rect,
                radius: [0.0, 0.0, 0.0, 0.0],
            } if clip_rect == rect(0.0, 0.0, 30.0, 40.0)
        ));
        assert_eq!(
            clip.transform.transform_point(point(0.0, 0.0)),
            point(1.0, 2.0)
        );
        assert_eq!(list.commands[0].clips, vec![clip.id]);
    }

    #[test]
    fn transform_rotate_records_affine_without_screen_space_points() {
        let mut tree = Tree::new();
        let child = tree.insert(leaf_node(
            "child",
            LeafKind::Line {
                start: point(0.0, 0.0),
                end: point(10.0, 0.0),
                stroke: Stroke::new(1.0, Color::WHITE),
            },
            rect(10.0, 10.0, 10.0, 10.0),
        ));
        let mut root_node = container_node("root", rect(0.0, 0.0, 100.0, 100.0), vec![child]);
        root_node.style.transform = Some(TransformSpec::translate_scale_rotate(
            [0.0, 0.0],
            1.0,
            std::f32::consts::FRAC_PI_2,
        ));
        let root = tree.insert(root_node);
        tree.set_root(root);

        let list = paint_tree(&tree, root);
        let command = only_command(&list);

        assert!(matches!(
            &command.command,
            PaintCommand::Path(path)
                if path.data == PathData::line(point(0.0, 0.0), point(10.0, 0.0))
        ));
        assert_eq!(
            command.transform.transform_point(point(0.0, 0.0)),
            point(-10.0, 10.0)
        );
    }

    #[derive(Debug)]
    struct TestCustomPainter;

    impl CustomPainter for TestCustomPainter {
        fn paint(&self, target: &mut dyn PaintTarget, cx: CustomPaintCx) {
            assert_eq!(cx.local_rect, rect(0.0, 0.0, 8.0, 10.0));
            target.draw_circle(
                Point {
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
                Point {
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
        let expected_transform =
            Affine2D::compose(expected_parent, Affine2D::translation(3.0, 4.0));
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

    #[test]
    fn svg_style_from_icon_preserves_icon_style_fields() {
        let style = IconStyle::monochrome(Color::WHITE)
            .with_fill(IconPaintOverride::Preserve)
            .with_stroke(IconPaintOverride::None)
            .with_stroke_width(IconStrokeWidth::SvgUnits(2.0))
            .with_fit(IconFit::Stretch);

        let svg = svg_style_from_icon(style);

        assert_eq!(svg.fill, SvgPaintOverride::Preserve);
        assert_eq!(svg.stroke, SvgPaintOverride::None);
        assert_eq!(svg.stroke_width, SvgStrokeWidth::SvgUnits(2.0));
        assert_eq!(svg.fit, SvgFit::Stretch);
    }

    #[test]
    fn decoration_records_local_rect_command() {
        let mut tree = Tree::new();
        let mut root_node = container_node("root", rect(10.0, 20.0, 30.0, 40.0), Vec::new());
        root_node.decoration = Some(super::super::layout::Decoration {
            background: Some(Color::BLACK),
            border: Some(Border {
                width: 2.0,
                color: Color::WHITE,
            }),
            radius: [3.0; 4],
            shadow: None,
        });
        let root = tree.insert(root_node);
        tree.set_root(root);

        let list = paint_tree(&tree, root);

        assert_eq!(
            only_command(&list).command,
            PaintCommand::Rect(RectPaint {
                rect: rect(0.0, 0.0, 30.0, 40.0),
                style: RectStyle {
                    color: Color::BLACK,
                    border: Some(Border {
                        width: 2.0,
                        color: Color::WHITE,
                    }),
                    radius: [3.0; 4],
                    shadow: None,
                },
            })
        );
    }
}
