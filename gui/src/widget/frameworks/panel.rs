use crate::icon::names;
use crate::renderer::{Border, Color, Rect, Shadow, TextStyle};
use crate::tree::layout::{Align, LeafKind, Overflow, Size};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::atoms::button::ButtonProps;
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

    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::Panel
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
            let mut titlebar_children = vec![ui::leaf(
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
            )
            .fill_width()
            .flex_shrink(1.0)
            .build()];
            if self.closable {
                titlebar_children.push(
                    ui::widget(anatomy.part("close"), ButtonProps::icon_only(names::XMARK)).build(),
                );
            }

            let mut titlebar = ui::row(anatomy.titlebar())
                .align_items(Align::Center)
                .gap(tokens.title_padding_x * 0.5)
                .fixed_height(tokens.title_bar_height)
                .padding_symmetric(tokens.title_padding_y, tokens.title_padding_x)
                .background(visual.titlebar_background)
                .radius([tokens.radius, tokens.radius, 0.0, 0.0])
                .children(titlebar_children);
            titlebar = titlebar.draggable(self.draggable);
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
        root = root.resizable(self.resizable);
        root.build()
    }
}

fn rect_eq(a: Rect, b: Rect) -> bool {
    a.x == b.x && a.y == b.y && a.w == b.w && a.h == b.h
}
