use crate::tree::layout::{BoxStyle, LeafKind, Position, Size};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct CanvasConnectionView {
    pub from_port_id: String,
    pub to_port_id: String,
}

pub fn connection_layer(connections: &[CanvasConnectionView]) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("canvas_connections"),
        style: BoxStyle {
            position: Position::Absolute { x: 0.0, y: 0.0 },
            width: Size::Fill,
            height: Size::Fill,
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children: connections
            .iter()
            .enumerate()
            .map(|(index, connection)| Desc::Leaf {
                id: Cow::Owned(format!("canvas_connection::{index}")),
                style: BoxStyle {
                    position: Position::Absolute { x: 0.0, y: 0.0 },
                    width: Size::Fixed(0.0),
                    height: Size::Fixed(0.0),
                    hittable: Some(false),
                    ..BoxStyle::default()
                },
                kind: LeafKind::Connection {
                    from_port: Cow::Owned(connection.from_port_id.clone()),
                    to_port: Cow::Owned(connection.to_port_id.clone()),
                },
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_layer_builds_connection_leaves() {
        let desc = connection_layer(&[CanvasConnectionView {
            from_port_id: "from".to_string(),
            to_port_id: "to".to_string(),
        }]);

        let Desc::Container { children, .. } = desc else {
            panic!("connection layer should be a container");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "canvas_connection::0");
    }
}
