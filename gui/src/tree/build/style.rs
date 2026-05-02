#[cfg(test)]
use crate::geometry::TransformSpec;
use crate::gesture::Gesture;
use crate::tree::build::{ContainerBuilder, LeafBuilder};
use crate::tree::layout::{Align, BoxStyle, Edges, Justify, Overflow, Position, Size};
use crate::widget::build::WidgetBuildBuilder;

pub trait StyleBuilder: Sized {
    fn style_mut(&mut self) -> &mut BoxStyle;

    fn style(mut self, style: BoxStyle) -> Self {
        *self.style_mut() = style;
        self
    }

    fn map_style(mut self, update: impl FnOnce(&mut BoxStyle)) -> Self {
        update(self.style_mut());
        self
    }

    fn width(self, width: Size) -> Self {
        self.map_style(|style| style.width = width)
    }

    fn height(self, height: Size) -> Self {
        self.map_style(|style| style.height = height)
    }

    fn fixed_width(self, width: f32) -> Self {
        self.width(Size::Fixed(width))
    }

    fn fixed_height(self, height: f32) -> Self {
        self.height(Size::Fixed(height))
    }

    fn fill_width(self) -> Self {
        self.width(Size::Fill)
    }

    fn fill_height(self) -> Self {
        self.height(Size::Fill)
    }

    fn auto_width(self) -> Self {
        self.width(Size::Auto)
    }

    fn auto_height(self) -> Self {
        self.height(Size::Auto)
    }

    #[cfg(test)]
    fn min_width(self, width: f32) -> Self {
        self.map_style(|style| style.min_width = width)
    }

    #[cfg(test)]
    fn max_width(self, width: f32) -> Self {
        self.map_style(|style| style.max_width = width)
    }

    fn min_height(self, height: f32) -> Self {
        self.map_style(|style| style.min_height = height)
    }

    #[cfg(test)]
    fn max_height(self, height: f32) -> Self {
        self.map_style(|style| style.max_height = height)
    }

    fn padding(self, padding: Edges) -> Self {
        self.map_style(|style| style.padding = padding)
    }

    fn padding_all(self, padding: f32) -> Self {
        self.padding(Edges::all(padding))
    }

    fn padding_symmetric(self, vertical: f32, horizontal: f32) -> Self {
        self.padding(Edges::symmetric(vertical, horizontal))
    }

    #[cfg(test)]
    fn margin(self, margin: Edges) -> Self {
        self.map_style(|style| style.margin = margin)
    }

    #[cfg(test)]
    fn margin_all(self, margin: f32) -> Self {
        self.margin(Edges::all(margin))
    }

    fn gap(self, gap: f32) -> Self {
        self.map_style(|style| style.gap = gap)
    }

    fn align_items(self, align: Align) -> Self {
        self.map_style(|style| style.align_items = align)
    }

    #[cfg(test)]
    fn align_self(self, align: Align) -> Self {
        self.map_style(|style| style.align_self = Some(align))
    }

    fn justify_content(self, justify: Justify) -> Self {
        self.map_style(|style| style.justify_content = justify)
    }

    fn flex_grow(self, flex_grow: f32) -> Self {
        self.map_style(|style| style.flex_grow = flex_grow)
    }

    fn flex_shrink(self, flex_shrink: f32) -> Self {
        self.map_style(|style| style.flex_shrink = flex_shrink)
    }

    fn overflow(self, overflow: Overflow) -> Self {
        self.map_style(|style| style.overflow = overflow)
    }

    #[cfg(test)]
    fn relative(self) -> Self {
        self.map_style(|style| style.position = Position::relative())
    }

    fn absolute_xy(self, x: f32, y: f32) -> Self {
        self.map_style(|style| style.position = Position::absolute_xy(x, y))
    }

    fn z_index(self, z_index: i32) -> Self {
        self.map_style(|style| style.z_index = z_index)
    }

    #[cfg(test)]
    fn transform(self, transform: TransformSpec) -> Self {
        self.map_style(|style| style.transform = Some(transform))
    }

    fn hittable(self, hittable: bool) -> Self {
        self.map_style(|style| style.hittable = Some(hittable))
    }

    fn draggable(self, draggable: bool) -> Self {
        self.map_style(|style| style.draggable = draggable)
    }

    fn resizable(self, resizable: bool) -> Self {
        self.map_style(|style| style.resizable = resizable)
    }

    #[cfg(test)]
    fn resize_edge_threshold(self, threshold: f32) -> Self {
        self.map_style(|style| style.resize_edge_threshold = threshold)
    }

    fn gesture(self, gesture: Gesture) -> Self {
        self.map_style(|style| style.gestures.push(gesture))
    }
}

impl StyleBuilder for ContainerBuilder {
    fn style_mut(&mut self) -> &mut BoxStyle {
        &mut self.style
    }
}

impl StyleBuilder for LeafBuilder {
    fn style_mut(&mut self) -> &mut BoxStyle {
        &mut self.style
    }
}

impl StyleBuilder for WidgetBuildBuilder {
    fn style_mut(&mut self) -> &mut BoxStyle {
        &mut self.style
    }
}
