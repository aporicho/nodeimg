use super::CanvasPendingConnectionView;
use crate::renderer::Point;
use crate::tree::layout::{BoxStyle, LeafKind, Position, Size};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct CanvasConnectionView {
    pub from_port_id: String,
    pub to_port_id: String,
}

pub fn connection_layer(
    connections: &[CanvasConnectionView],
    pending: Option<&CanvasPendingConnectionView>,
) -> Desc {
    let mut children = connections
        .iter()
        .enumerate()
        .map(|(index, connection)| Desc::Leaf {
            id: Cow::Owned(format!("canvas_connection::{index}")),
            style: connection_leaf_style(),
            kind: LeafKind::Connection {
                from_port: Cow::Owned(connection.from_port_id.clone()),
                to_port: Cow::Owned(connection.to_port_id.clone()),
            },
        })
        .collect::<Vec<_>>();

    if let Some(pending) = pending {
        children.push(Desc::Leaf {
            id: Cow::Borrowed("canvas_connection::pending"),
            style: connection_leaf_style(),
            kind: LeafKind::PendingConnection {
                from_port: Cow::Owned(pending.from_port_id.clone()),
                cursor_canvas: Point {
                    x: pending.cursor_canvas[0],
                    y: pending.cursor_canvas[1],
                },
            },
        });
    }

    Desc::Container {
        id: Cow::Borrowed("canvas_connections"),
        style: BoxStyle {
            position: Position::absolute_xy(0.0, 0.0),
            width: Size::Fill,
            height: Size::Fill,
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn connection_leaf_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fixed(0.0),
        height: Size::Fixed(0.0),
        hittable: Some(false),
        ..BoxStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_layer_builds_connection_leaves() {
        let desc = connection_layer(
            &[CanvasConnectionView {
                from_port_id: "from".to_string(),
                to_port_id: "to".to_string(),
            }],
            None,
        );

        let Desc::Container { children, .. } = desc else {
            panic!("connection layer should be a container");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "canvas_connection::0");
    }

    #[test]
    fn connection_layer_can_include_pending_connection() {
        let desc = connection_layer(
            &[],
            Some(&CanvasPendingConnectionView {
                from_port_id: "from".to_string(),
                cursor_canvas: [40.0, 50.0],
            }),
        );

        let Desc::Container { children, .. } = desc else {
            panic!("connection layer should be a container");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "canvas_connection::pending");
    }
}
