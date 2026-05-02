use crate::renderer::{Border, Color, Shadow};
use crate::tree::build::ContainerBuilder;
use crate::tree::layout::Decoration;
use crate::widget::build::WidgetBuildBuilder;

pub trait DecorationBuilder: Sized {
    fn decoration_mut(&mut self) -> &mut Option<Decoration>;

    fn map_decoration(mut self, update: impl FnOnce(&mut Decoration)) -> Self {
        let decoration = self.decoration_mut().get_or_insert_with(empty_decoration);
        update(decoration);
        self
    }

    fn background(self, color: Color) -> Self {
        self.map_decoration(|decoration| decoration.background = Some(color))
    }

    fn border(self, border: Border) -> Self {
        self.map_decoration(|decoration| decoration.border = Some(border))
    }

    fn radius(self, radius: [f32; 4]) -> Self {
        self.map_decoration(|decoration| decoration.radius = radius)
    }

    fn radius_all(self, radius: f32) -> Self {
        self.radius([radius; 4])
    }

    fn shadow(self, shadow: Shadow) -> Self {
        self.map_decoration(|decoration| decoration.shadow = Some(shadow))
    }
}

impl DecorationBuilder for ContainerBuilder {
    fn decoration_mut(&mut self) -> &mut Option<Decoration> {
        &mut self.decoration
    }
}

impl DecorationBuilder for WidgetBuildBuilder {
    fn decoration_mut(&mut self) -> &mut Option<Decoration> {
        &mut self.decoration
    }
}

fn empty_decoration() -> Decoration {
    Decoration {
        background: None,
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Color;
    use crate::tree::build::container;
    use crate::tree::Desc;

    #[test]
    fn decoration_is_created_lazily() {
        let desc = container("surface").background(Color::WHITE).build();

        let Desc::Container { decoration, .. } = desc else {
            panic!("expected container");
        };
        let decoration = decoration.expect("expected decoration");
        assert_eq!(decoration.background, Some(Color::WHITE));
        assert_eq!(decoration.radius, [0.0; 4]);
    }
}
