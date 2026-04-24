use super::catalog::PlaygroundItemId;
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
use std::borrow::Cow;

pub(super) fn primitive_sample(
    item: PlaygroundItemId,
    theme: &Theme,
    image: TextureHandle,
) -> Desc {
    match item {
        PlaygroundItemId::Surface => surface_sample(theme),
        PlaygroundItemId::Text => text_sample(theme),
        PlaygroundItemId::Image => image_sample(theme, image),
        PlaygroundItemId::Icon => icon_sample(theme),
        PlaygroundItemId::Circle => circle_sample(theme),
        PlaygroundItemId::Line => line_sample(theme),
        PlaygroundItemId::Curve => curve_sample(theme),
        PlaygroundItemId::Path => path_sample(theme),
        PlaygroundItemId::Grid => grid_sample(theme),
        PlaygroundItemId::Connection => connection_sample(theme),
        _ => empty_sample(item),
    }
}

fn sample_stage(id: impl Into<Cow<'static, str>>, theme: &Theme) -> gui::ui::ContainerBuilder {
    ui::container(id)
        .relative()
        .fill_width()
        .fill_height()
        .overflow(Overflow::Hidden)
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
}

fn surface_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_surface_sample", theme)
        .child(
            ui::container("primitive_surface_rect")
                .absolute_xy(12.0, 11.0)
                .fixed_width(48.0)
                .fixed_height(38.0)
                .background(theme.colors.accent)
                .border(Border {
                    width: 2.0,
                    color: theme.colors.border_focus,
                })
                .build(),
        )
        .child(
            ui::container("primitive_surface_radius")
                .absolute_xy(75.0, 11.0)
                .fixed_width(50.0)
                .fixed_height(38.0)
                .background(theme.colors.accent_soft)
                .border(Border {
                    width: 1.5,
                    color: theme.colors.accent,
                })
                .radius([8.0, 2.0, 8.0, 2.0])
                .build(),
        )
        .child(
            ui::container("primitive_surface_shadow")
                .absolute_xy(142.0, 9.0)
                .fixed_width(48.0)
                .fixed_height(34.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.border,
                })
                .radius_all(8.0)
                .shadow(Shadow {
                    color: rgba(0.0, 0.0, 0.0, 0.26),
                    offset: [0.0, 6.0],
                    blur: 12.0,
                    spread: 1.0,
                })
                .build(),
        )
        .build()
}

fn text_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_text_sample", theme)
        .child(
            ui::text_with_layout(
                "primitive_text_ellipsis",
                "Long text trimmed with ellipsis",
                theme.text_style_body_md(),
                TextLayout {
                    overflow: TextOverflow::Ellipsis,
                    align: TextAlign::Start,
                },
            )
            .absolute_xy(10.0, 8.0)
            .fixed_width(104.0)
            .fixed_height(20.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "primitive_text_center",
                "Center",
                TextStyle {
                    color: theme.colors.accent,
                    ..theme.text_style_title_sm()
                },
                TextLayout {
                    overflow: TextOverflow::Clip,
                    align: TextAlign::Center,
                },
            )
            .absolute_xy(120.0, 8.0)
            .fixed_width(72.0)
            .fixed_height(22.0)
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
            .absolute_xy(10.0, 35.0)
            .fixed_width(182.0)
            .fixed_height(20.0)
            .build(),
        )
        .build()
}

fn image_sample(theme: &Theme, image: TextureHandle) -> Desc {
    sample_stage("primitive_image_sample", theme)
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
            .absolute_xy(14.0, 8.0)
            .fixed_width(70.0)
            .fixed_height(46.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_image_tint",
                LeafKind::Image {
                    texture: image,
                    style: ImageStyle::default()
                        .with_fit(ImageFit::Contain)
                        .with_tint(rgba(1.0, 0.72, 0.36, 1.0))
                        .with_opacity(ImageOpacity::new(0.78)),
                },
            )
            .absolute_xy(116.0, 8.0)
            .fixed_width(70.0)
            .fixed_height(46.0)
            .build(),
        )
        .build()
}

fn icon_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_icon_sample", theme)
        .child(
            ui::icon(
                "primitive_icon_settings",
                names::SETTINGS,
                34.0,
                theme.colors.text,
            )
            .absolute_xy(30.0, 14.0)
            .build(),
        )
        .child(
            ui::icon(
                "primitive_icon_check",
                names::CHECK,
                34.0,
                theme.colors.accent,
            )
            .absolute_xy(86.0, 14.0)
            .build(),
        )
        .child(
            ui::icon(
                "primitive_icon_arrow",
                names::NAV_ARROW_RIGHT,
                34.0,
                theme.colors.text_muted,
            )
            .absolute_xy(142.0, 14.0)
            .build(),
        )
        .build()
}

fn circle_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_circle_sample", theme)
        .child(
            ui::leaf(
                "primitive_circle_filled",
                LeafKind::Circle {
                    radius: 18.0,
                    fill: Some(theme.colors.accent_soft),
                    stroke: Some(Border {
                        width: 2.0,
                        color: theme.colors.accent,
                    }),
                },
            )
            .absolute_xy(24.0, 12.0)
            .fixed_width(42.0)
            .fixed_height(42.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_circle_outline",
                LeafKind::Circle {
                    radius: 21.0,
                    fill: None,
                    stroke: Some(Border {
                        width: 3.0,
                        color: theme.colors.text,
                    }),
                },
            )
            .absolute_xy(118.0, 9.0)
            .fixed_width(48.0)
            .fixed_height(48.0)
            .build(),
        )
        .build()
}

fn line_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_line_sample", theme)
        .child(
            ui::line(
                "primitive_line_round",
                point(12.0, 42.0),
                point(176.0, 12.0),
                Stroke::new(4.0, theme.colors.accent).with_cap(LineCap::Round),
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(196.0)
            .fixed_height(62.0)
            .build(),
        )
        .child(
            ui::line(
                "primitive_line_square",
                point(18.0, 18.0),
                point(188.0, 46.0),
                Stroke::new(2.0, theme.colors.text_muted).with_cap(LineCap::Square),
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(196.0)
            .fixed_height(62.0)
            .build(),
        )
        .build()
}

fn curve_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_curve_sample", theme)
        .child(
            ui::curve(
                "primitive_curve_bezier",
                [
                    point(12.0, 46.0),
                    point(56.0, 0.0),
                    point(116.0, 72.0),
                    point(184.0, 18.0),
                ],
                Stroke::new(3.0, theme.colors.accent)
                    .with_cap(LineCap::Round)
                    .with_join(LineJoin::Round),
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(196.0)
            .fixed_height(62.0)
            .build(),
        )
        .build()
}

fn path_sample(theme: &Theme) -> Desc {
    let path = PathData::new()
        .move_to(point(24.0, 48.0))
        .line_to(point(50.0, 10.0))
        .line_to(point(78.0, 48.0))
        .close()
        .move_to(point(42.0, 38.0))
        .line_to(point(50.0, 24.0))
        .line_to(point(58.0, 38.0))
        .close();

    sample_stage("primitive_path_sample", theme)
        .child(
            ui::path(
                "primitive_path_even_odd",
                path,
                PathStyle::fill_and_stroke(
                    Fill::even_odd(theme.colors.accent_soft),
                    Stroke::new(2.0, theme.colors.text),
                ),
            )
            .absolute_xy(52.0, 4.0)
            .fixed_width(92.0)
            .fixed_height(58.0)
            .transform(TransformSpec::translate_scale_rotate(
                [0.0, 0.0],
                1.0,
                -0.08,
            ))
            .build(),
        )
        .build()
}

fn grid_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_grid_sample", theme)
        .child(
            ui::leaf(
                "primitive_grid_leaf",
                LeafKind::Grid {
                    spacing: 12.0,
                    dot_color: theme.colors.canvas_grid,
                    dot_size: 1.4,
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(204.0)
            .fixed_height(70.0)
            .build(),
        )
        .build()
}

fn connection_sample(theme: &Theme) -> Desc {
    sample_stage("primitive_connection_sample", theme)
        .child(port("primitive_connection_from", 22.0, 36.0, theme))
        .child(port("primitive_connection_to", 170.0, 18.0, theme))
        .child(
            ui::leaf(
                "primitive_connection_curve",
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
        .build()
}

fn port(id: &'static str, x: f32, y: f32, theme: &Theme) -> Desc {
    ui::leaf(
        id,
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(theme.colors.accent),
            stroke: Some(Border {
                width: 1.0,
                color: theme.colors.text,
            }),
        },
    )
    .absolute_xy(x, y)
    .fixed_width(10.0)
    .fixed_height(10.0)
    .build()
}

fn empty_sample(item: PlaygroundItemId) -> Desc {
    ui::container(format!("primitive_empty_{}", item.key())).build()
}

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
}
