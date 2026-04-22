use super::CanvasPendingConnectionView;
use crate::renderer::Point;
use crate::tree::layout::LeafKind;
use crate::tree::Desc;
use crate::ui::{self, StyleBuilder};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct CanvasConnectionView {
    pub from_port_id: String,
    pub to_port_id: String,
}

pub fn connection_layer(
    connections: &[CanvasConnectionView],
    pending: Option<&CanvasPendingConnectionView>,
    z_index: i32,
) -> Desc {
    let mut children = connections
        .iter()
        .enumerate()
        .map(|(index, connection)| {
            connection_leaf(
                format!("canvas_connection::{index}"),
                LeafKind::Connection {
                    from_port: Cow::Owned(connection.from_port_id.clone()),
                    to_port: Cow::Owned(connection.to_port_id.clone()),
                },
            )
        })
        .collect::<Vec<_>>();

    if let Some(pending) = pending {
        children.push(connection_leaf(
            "canvas_connection::pending",
            LeafKind::PendingConnection {
                from_port: Cow::Owned(pending.from_port_id.clone()),
                cursor_canvas: Point {
                    x: pending.cursor_canvas[0],
                    y: pending.cursor_canvas[1],
                },
            },
        ));
    }

    ui::container("canvas_connections")
        .absolute_xy(0.0, 0.0)
        .z_index(z_index)
        .fill_width()
        .fill_height()
        .hittable(false)
        .children(children)
        .build()
}

fn connection_leaf(id: impl Into<Cow<'static, str>>, kind: LeafKind) -> Desc {
    ui::leaf(id, kind)
        .absolute_xy(0.0, 0.0)
        .fixed_width(0.0)
        .fixed_height(0.0)
        .hittable(false)
        .build()
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
            -10,
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
            -10,
        );

        let Desc::Container { children, .. } = desc else {
            panic!("connection layer should be a container");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "canvas_connection::pending");
    }
}
