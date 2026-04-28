use std::collections::{BTreeSet, HashMap};

use crate::geometry::Point;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasEndpointCache {
    endpoints: HashMap<String, Point>,
    edges_by_endpoint: HashMap<String, BTreeSet<String>>,
}

impl CanvasEndpointCache {
    pub fn update_endpoint(
        &mut self,
        endpoint_id: impl Into<String>,
        point: Point,
        connected_edges: impl IntoIterator<Item = impl Into<String>>,
    ) -> bool {
        let endpoint_id = endpoint_id.into();
        let before = self.endpoints.insert(endpoint_id.clone(), point);
        let edges = connected_edges
            .into_iter()
            .map(Into::into)
            .collect::<BTreeSet<_>>();
        let old_edges = self.edges_by_endpoint.insert(endpoint_id, edges);
        before != Some(point) || old_edges.is_none()
    }

    pub fn endpoint(&self, endpoint_id: &str) -> Option<Point> {
        self.endpoints.get(endpoint_id).copied()
    }

    pub fn remove_endpoint(&mut self, endpoint_id: &str) -> Option<Point> {
        self.edges_by_endpoint.remove(endpoint_id);
        self.endpoints.remove(endpoint_id)
    }

    pub fn affected_edges_for_endpoint(&self, endpoint_id: &str) -> impl Iterator<Item = &str> {
        self.edges_by_endpoint
            .get(endpoint_id)
            .into_iter()
            .flat_map(|edges| edges.iter().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_cache_tracks_affected_edges() {
        let mut cache = CanvasEndpointCache::default();

        assert!(cache.update_endpoint(
            "node::out",
            Point { x: 10.0, y: 20.0 },
            ["edge::a", "edge::b"]
        ));

        assert_eq!(
            cache.endpoint("node::out"),
            Some(Point { x: 10.0, y: 20.0 })
        );
        assert_eq!(
            cache
                .affected_edges_for_endpoint("node::out")
                .collect::<Vec<_>>(),
            vec!["edge::a", "edge::b"]
        );
        assert_eq!(
            cache.remove_endpoint("node::out"),
            Some(Point { x: 10.0, y: 20.0 })
        );
        assert!(cache
            .affected_edges_for_endpoint("node::out")
            .next()
            .is_none());
    }
}
