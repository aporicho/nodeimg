"""Graph executor for the Python backend.

Parses a graph definition (nodes, connections, output_node), performs
topological sorting via Kahn's algorithm, and executes each node in order.
"""

from __future__ import annotations

import logging
import time
from collections import defaultdict, deque
from typing import Any

from registry import NodeRegistry

log = logging.getLogger("executor")


class GraphExecutor:
    """Executes a node graph given a registry of node definitions."""

    def __init__(self, registry: NodeRegistry) -> None:
        self._registry = registry

    def execute(self, graph: dict[str, Any]) -> dict[str, Any]:
        """Execute a full graph and return the outputs of all nodes.

        Parameters
        ----------
        graph : dict
            Must contain:
            - "nodes": dict mapping node_id -> {"type": str, "params": dict}
            - "connections": list of {"from_node", "from_output", "to_node", "to_input"}
            - "output_node": node_id of the final output node

        Returns
        -------
        dict with key "outputs" mapping node_id -> {output_name: value, ...}
        """
        nodes = graph["nodes"]
        connections = graph.get("connections", [])
        output_node = graph.get("output_node")

        log.info(
            "Graph received: %d nodes, %d connections, output_node=%s",
            len(nodes),
            len(connections),
            output_node,
        )
        for nid, info in nodes.items():
            log.info(
                "  node %s: type=%s params=%s",
                nid,
                info["type"],
                list(info.get("params", {}).keys()),
            )

        # Build adjacency info: for each node, which inputs come from where
        # input_map[to_node][to_input] = (from_node, from_output)
        input_map: dict[str, dict[str, tuple[str, str]]] = defaultdict(dict)
        for conn in connections:
            input_map[conn["to_node"]][conn["to_input"]] = (
                conn["from_node"],
                conn["from_output"],
            )

        # Topological sort
        sorted_ids = self._topo_sort(nodes, connections)
        log.info("Execution order: %s", sorted_ids)

        # Execute in topological order
        results: dict[str, dict[str, Any]] = {}
        total_start = time.time()
        for node_id in sorted_ids:
            node_info = nodes[node_id]
            node_type = node_info["type"]
            node_def = self._registry.get(node_type)

            # Gather inputs from upstream results
            inputs: dict[str, Any] = {}
            for mapping_input, (src_node, src_output) in input_map.get(
                node_id, {}
            ).items():
                inputs[mapping_input] = results[src_node][src_output]

            # Execute the node
            params = node_info.get("params", {})
            log.info(
                "Executing node %s (%s) inputs=%s",
                node_id,
                node_type,
                list(inputs.keys()),
            )
            t0 = time.time()
            try:
                outputs = node_def.execute(None, inputs, params)
            except Exception:
                log.exception("Node %s (%s) failed", node_id, node_type)
                raise
            elapsed = time.time() - t0
            log.info(
                "Node %s (%s) done in %.2fs, outputs=%s",
                node_id,
                node_type,
                elapsed,
                list(outputs.keys()),
            )
            results[node_id] = outputs

        total_elapsed = time.time() - total_start
        log.info("Graph execution complete in %.2fs", total_elapsed)

        # Only return the output node's results (intermediate nodes contain
        # non-serializable objects like torch models/tensors).
        if not output_node:
            raise ValueError("'output_node' is required in the graph definition")
        if output_node not in results:
            raise KeyError(
                f"output_node '{output_node}' not found in execution results"
            )
        return {"outputs": {output_node: results[output_node]}}

    @staticmethod
    def _topo_sort(
        nodes: dict[str, Any],
        connections: list[dict[str, str]],
    ) -> list[str]:
        """Topological sort using Kahn's algorithm.

        Raises ValueError if a cycle is detected.
        """
        # Build in-degree map and adjacency list
        in_degree: dict[str, int] = {nid: 0 for nid in nodes}
        adjacency: dict[str, list[str]] = {nid: [] for nid in nodes}

        for conn in connections:
            from_node = conn["from_node"]
            to_node = conn["to_node"]
            adjacency[from_node].append(to_node)
            in_degree[to_node] += 1

        # Start with nodes that have no incoming edges
        queue: deque[str] = deque()
        for nid, deg in in_degree.items():
            if deg == 0:
                queue.append(nid)

        sorted_ids: list[str] = []
        while queue:
            node_id = queue.popleft()
            sorted_ids.append(node_id)
            for neighbor in adjacency[node_id]:
                in_degree[neighbor] -= 1
                if in_degree[neighbor] == 0:
                    queue.append(neighbor)

        if len(sorted_ids) != len(nodes):
            raise ValueError("Graph contains a cycle")

        return sorted_ids
