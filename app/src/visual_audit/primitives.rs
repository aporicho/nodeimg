use super::section::VisualAuditSection;
use gui::geometry::TransformSpec;
use gui::icon::names;
use gui::paint::{Fill, ImageFilter, ImageFit, ImageOpacity, ImageSourceRect, LineCap, LineJoin};
use gui::renderer::{
    Border, Color, ImageStyle, PathData, PathStyle, Point, Shadow, Stroke, TextStyle,
};
use gui::theme::Theme;
use gui::tree::layout::{LeafKind, Overflow, TextAlign, TextLayout, TextOverflow, TextureHandle};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use std::borrow::Cow;

pub(super) fn primitive_section(theme: &Theme, image: TextureHandle) -> VisualAuditSection {
    VisualAuditSection {
        id: "primitives",
        title: "Primitive Graphics",
        column: 0,
        estimated_height: 760.0,
        content: vec![
            primitive_card(
                theme,
                "primitive_rect_card",
                "Rect / radius / shadow",
                primitive_rect_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_text_card",
                "Text clip / ellipsis / align",
                primitive_text_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_media_card",
                "Image / SVG icon",
                primitive_media_stage(theme, image),
            ),
            primitive_card(
                theme,
                "primitive_vector_card",
                "Circle / line / curve / path",
                primitive_vector_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_grid_connection_card",
                "Grid / connection / transform",
                primitive_grid_connection_stage(theme),
            ),
        ],
    }
}

fn primitive_card(theme: &Theme, id: &'static str, title: &'static str, stage: Desc) -> Desc {
    ui::column(id)
        .fill_width()
        .auto_height()
        .gap(theme.spacing.xs)
        .child(Desc::from(ui::widget(
            Cow::Owned(format!("{id}::label")),
            LabelProps {
                text: Cow::Borrowed(title),
                variant: LabelVariant::Caption,
                muted: true,
            },
        )))
        .child(stage)
        .build()
}

fn primitive_stage(
    theme: &Theme,
    id: &'static str,
    width: f32,
    height: f32,
) -> gui::tree::build::ContainerBuilder {
    ui::container(id)
        .relative()
        .fixed_width(width)
        .fixed_height(height)
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(theme.radii.sm.min(8.0))
        .overflow(Overflow::Hidden)
}

fn primitive_rect_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_rect_stage", 286.0, 76.0)
        .child(
            ui::container("primitive_rect_square")
                .absolute_xy(14.0, 16.0)
                .fixed_width(48.0)
                .fixed_height(44.0)
                .background(theme.colors.accent)
                .border(Border {
                    width: 2.0,
                    color: theme.colors.border_focus,
                })
                .build(),
        )
        .child(
            ui::container("primitive_rect_radius")
                .absolute_xy(80.0, 16.0)
                .fixed_width(58.0)
                .fixed_height(44.0)
                .background(theme.colors.accent_soft)
                .border(Border {
                    width: 1.5,
                    color: theme.colors.accent,
                })
                .radius([8.0, 2.0, 8.0, 2.0])
                .build(),
        )
        .child(
            ui::container("primitive_rect_shadow")
                .absolute_xy(158.0, 14.0)
                .fixed_width(92.0)
                .fixed_height(42.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.border,
                })
                .radius_all(8.0)
                .shadow(Shadow {
                    color: color(0.0, 0.0, 0.0, 0.28),
                    offset: [0.0, 8.0],
                    blur: 16.0,
                    spread: 1.0,
                })
                .build(),
        )
        .build()
}

fn primitive_text_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_text_stage", 286.0, 96.0)
        .child(
            ui::text_with_layout(
                "primitive_text_ellipsis",
                "Ellipsis should trim this long primitive text",
                theme.text_style_body_md(),
                TextLayout {
                    overflow: TextOverflow::Ellipsis,
                    align: TextAlign::Start,
                },
            )
            .absolute_xy(12.0, 12.0)
            .fixed_width(138.0)
            .fixed_height(22.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "primitive_text_center",
                "Centered",
                TextStyle {
                    color: theme.colors.accent,
                    ..theme.text_style_title_sm()
                },
                TextLayout {
                    overflow: TextOverflow::Clip,
                    align: TextAlign::Center,
                },
            )
            .absolute_xy(158.0, 12.0)
            .fixed_width(96.0)
            .fixed_height(24.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "primitive_text_end",
                "Right aligned",
                theme.text_style_mono_md(),
                TextLayout {
                    overflow: TextOverflow::Clip,
                    align: TextAlign::End,
                },
            )
            .absolute_xy(12.0, 50.0)
            .fixed_width(242.0)
            .fixed_height(22.0)
            .build(),
        )
        .build()
}

fn primitive_media_stage(theme: &Theme, image: TextureHandle) -> Desc {
    primitive_stage(theme, "primitive_media_stage", 286.0, 92.0)
        .child(
            ui::leaf(
                "primitive_image_cover",
                LeafKind::Image {
                    texture: image,
                    style: ImageStyle::default()
                        .with_fit(ImageFit::Cover)
                        .with_filter(ImageFilter::Nearest)
                        .with_source(ImageSourceRect::new(0.1, 0.1, 0.8, 0.8)),
                },
            )
            .absolute_xy(12.0, 12.0)
            .fixed_width(74.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_image_tint",
                LeafKind::Image {
                    texture: image,
                    style: ImageStyle::default()
                        .with_fit(ImageFit::Contain)
                        .with_tint(color(1.0, 0.72, 0.36, 1.0))
                        .with_opacity(ImageOpacity::new(0.78)),
                },
            )
            .absolute_xy(104.0, 12.0)
            .fixed_width(74.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::icon(
                "primitive_svg_icon",
                names::SETTINGS,
                36.0,
                theme.colors.text,
            )
            .absolute_xy(210.0, 22.0)
            .build(),
        )
        .build()
}

fn primitive_vector_stage(theme: &Theme) -> Desc {
    let stroke = Stroke::new(3.0, theme.colors.accent)
        .with_cap(LineCap::Round)
        .with_join(LineJoin::Round);
    let path = PathData::new()
        .move_to(point(14.0, 54.0))
        .line_to(point(34.0, 18.0))
        .line_to(point(58.0, 54.0))
        .close()
        .move_to(point(28.0, 42.0))
        .line_to(point(36.0, 28.0))
        .line_to(point(44.0, 42.0))
        .close();

    primitive_stage(theme, "primitive_vector_stage", 286.0, 118.0)
        .child(
            ui::leaf(
                "primitive_circle",
                LeafKind::Circle {
                    radius: 18.0,
                    fill: Some(theme.colors.accent_soft),
                    stroke: Some(Border {
                        width: 2.0,
                        color: theme.colors.accent,
                    }),
                },
            )
            .absolute_xy(14.0, 16.0)
            .fixed_width(42.0)
            .fixed_height(42.0)
            .build(),
        )
        .child(
            ui::line(
                "primitive_line",
                point(0.0, 22.0),
                point(74.0, 6.0),
                stroke.with_cap(LineCap::Square),
            )
            .absolute_xy(74.0, 16.0)
            .fixed_width(78.0)
            .fixed_height(42.0)
            .build(),
        )
        .child(
            ui::curve(
                "primitive_curve",
                [
                    point(0.0, 34.0),
                    point(22.0, 0.0),
                    point(42.0, 58.0),
                    point(68.0, 18.0),
                ],
                stroke,
            )
            .absolute_xy(174.0, 10.0)
            .fixed_width(76.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::path(
                "primitive_path_even_odd",
                path,
                PathStyle::fill_and_stroke(
                    Fill::even_odd(theme.colors.accent_soft),
                    Stroke::new(2.0, theme.colors.text),
                ),
            )
            .absolute_xy(92.0, 60.0)
            .fixed_width(72.0)
            .fixed_height(56.0)
            .transform(TransformSpec::translate_scale_rotate([0.0, 0.0], 1.0, 0.12))
            .build(),
        )
        .build()
}

fn primitive_grid_connection_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_grid_connection_stage", 286.0, 118.0)
        .child(
            ui::leaf(
                "primitive_grid",
                LeafKind::Grid {
                    spacing: 12.0,
                    dot_color: theme.colors.canvas_grid,
                    dot_size: 1.4,
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(286.0)
            .fixed_height(118.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection_from",
                LeafKind::Circle {
                    radius: 5.0,
                    fill: Some(theme.colors.accent),
                    stroke: Some(Border {
                        width: 1.0,
                        color: theme.colors.text,
                    }),
                },
            )
            .absolute_xy(28.0, 54.0)
            .fixed_width(10.0)
            .fixed_height(10.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection_to",
                LeafKind::Circle {
                    radius: 5.0,
                    fill: Some(theme.colors.accent),
                    stroke: Some(Border {
                        width: 1.0,
                        color: theme.colors.text,
                    }),
                },
            )
            .absolute_xy(234.0, 34.0)
            .fixed_width(10.0)
            .fixed_height(10.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection",
                LeafKind::Connection {
                    from_port: Cow::Borrowed("primitive_connection_from"),
                    to_port: Cow::Borrowed("primitive_connection_to"),
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(0.0)
            .fixed_height(0.0)
            .build(),
        )
        .child(
            ui::container("primitive_transform_marker")
                .absolute_xy(118.0, 68.0)
                .fixed_width(42.0)
                .fixed_height(22.0)
                .background(theme.colors.accent_soft)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.accent,
                })
                .radius_all(5.0)
                .transform(TransformSpec::translate_scale_rotate(
                    [0.0, 0.0],
                    1.0,
                    -0.35,
                ))
                .build(),
        )
        .build()
}

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn color(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
}
