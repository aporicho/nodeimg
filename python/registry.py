"""Node registry for the Python backend.

Defines dataclasses for node definitions (pins, params) and a registry
that stores node types by name.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Optional


@dataclass
class PinDef:
    """Definition of a single input or output pin on a node."""

    name: str
    type: str  # e.g. "IMAGE", "INT", "FLOAT", "STRING", "LATENT", "MODEL", ...


@dataclass
class ParamDef:
    """Definition of a user-configurable parameter on a node."""

    name: str
    type: str  # "INT", "FLOAT", "STRING", "BOOL", "ENUM"
    default: Any = None
    min: Optional[float] = None
    max: Optional[float] = None
    options: Optional[list[str]] = None  # for ENUM type
    widget: Optional[str] = None  # UI hint: "slider", "dropdown", "text", ...


@dataclass
class NodeDef:
    """Full definition of a node type."""

    inputs: list[PinDef] = field(default_factory=list)
    outputs: list[PinDef] = field(default_factory=list)
    params: list[ParamDef] = field(default_factory=list)
    execute: Callable[[dict, dict], dict] = lambda inputs, params: {}


class NodeRegistry:
    """Registry that maps node type names to their definitions."""

    def __init__(self) -> None:
        self._nodes: dict[str, NodeDef] = {}

    def register(self, name: str, node_def: NodeDef) -> None:
        """Register a node type. Raises ValueError if already registered."""
        if name in self._nodes:
            raise ValueError(f"Node type '{name}' is already registered")
        self._nodes[name] = node_def

    def get(self, name: str) -> NodeDef:
        """Retrieve a node definition by name. Raises KeyError if not found."""
        if name not in self._nodes:
            raise KeyError(f"Unknown node type: '{name}'")
        return self._nodes[name]

    def list_all(self) -> dict[str, Any]:
        """Return all registered node types as a JSON-serializable dict."""
        result: dict[str, Any] = {}
        for name, node_def in self._nodes.items():
            result[name] = {
                "inputs": [
                    {"name": pin.name, "type": pin.type}
                    for pin in node_def.inputs
                ],
                "outputs": [
                    {"name": pin.name, "type": pin.type}
                    for pin in node_def.outputs
                ],
                "params": [
                    {
                        "name": p.name,
                        "type": p.type,
                        "default": p.default,
                        "min": p.min,
                        "max": p.max,
                        "options": p.options,
                        "widget": p.widget,
                    }
                    for p in node_def.params
                ],
            }
        return result
