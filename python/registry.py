"""Node declaration and registry support for the Python backend.

Provides the new declaration style:
- @node(...)
- Pin(...)
- Param(...)

And a runtime registry that can load definitions from python/nodes modules.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Optional
import importlib
import pkgutil


@dataclass
class Pin:
    name: str
    data_type: str
    required: bool = True


@dataclass
class Param:
    name: str
    data_type: str
    default: Any
    expose: list[str]
    min: Optional[float] = None
    max: Optional[float] = None
    options: Optional[list[str]] = None
    widget: Optional[str] = None


@dataclass
class NodeSpec:
    type_id: str
    title: str
    category: str
    inputs: list[Pin] = field(default_factory=list)
    outputs: list[Pin] = field(default_factory=list)
    params: list[Param] = field(default_factory=list)
    execute: Optional[Callable[[Any, dict, dict], dict]] = None


def node(
    *,
    type_id: str,
    title: str,
    category: str,
    inputs: list[Pin],
    outputs: list[Pin],
    params: list[Param],
):
    def decorator(func: Callable[[Any, dict, dict], dict]):
        func.__node_spec__ = NodeSpec(
            type_id=type_id,
            title=title,
            category=category,
            inputs=inputs,
            outputs=outputs,
            params=params,
            execute=func,
        )
        return func

    return decorator


class NodeRegistry:
    def __init__(self) -> None:
        self._nodes: dict[str, NodeSpec] = {}

    def register(self, spec: NodeSpec) -> None:
        if spec.type_id in self._nodes:
            raise ValueError(f"Node type '{spec.type_id}' is already registered")
        self._nodes[spec.type_id] = spec

    def get(self, type_id: str) -> NodeSpec:
        if type_id not in self._nodes:
            raise KeyError(f"Unknown node type: '{type_id}'")
        return self._nodes[type_id]

    def list_all(self) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for type_id, spec in self._nodes.items():
            result[type_id] = {
                "title": spec.title,
                "category": spec.category,
                "inputs": [
                    {
                        "name": pin.name,
                        "type": pin.data_type,
                        "required": pin.required,
                    }
                    for pin in spec.inputs
                ],
                "outputs": [
                    {
                        "name": pin.name,
                        "type": pin.data_type,
                    }
                    for pin in spec.outputs
                ],
                "params": [
                    {
                        "name": param.name,
                        "type": param.data_type,
                        "default": param.default,
                        "min": param.min,
                        "max": param.max,
                        "options": param.options,
                        "widget": param.widget,
                        "expose": param.expose,
                    }
                    for param in spec.params
                ],
            }
        return result

    def register_package(self, package_name: str) -> None:
        package = importlib.import_module(package_name)
        for module_info in pkgutil.iter_modules(package.__path__):
            if module_info.name.startswith("__"):
                continue
            module = importlib.import_module(f"{package_name}.{module_info.name}")
            for value in module.__dict__.values():
                spec = getattr(value, "__node_spec__", None)
                if spec is not None:
                    self.register(spec)
