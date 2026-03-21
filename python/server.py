"""FastAPI server for the Node Image Studio Python backend.

Endpoints:
- GET  /health      — GPU status and availability
- GET  /node_types  — list all registered node types
- POST /execute     — execute a graph and return results
"""

from __future__ import annotations

import logging
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import Any

from registry import NodeRegistry
from executor import GraphExecutor
from nodes import register_all

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("server")

# ---------------------------------------------------------------------------
# Global instances
# ---------------------------------------------------------------------------

registry = NodeRegistry()
register_all(registry)
executor = GraphExecutor(registry)

app = FastAPI(title="Node Image Studio Backend")


# ---------------------------------------------------------------------------
# Request / Response models
# ---------------------------------------------------------------------------

class ExecuteRequest(BaseModel):
    graph: dict[str, Any]


# ---------------------------------------------------------------------------
# Endpoints
# ---------------------------------------------------------------------------

@app.get("/health")
def health() -> dict[str, Any]:
    """Return backend health status including GPU info."""
    gpu_name: str | None = None
    vram_free_gb: float | None = None

    try:
        import torch
        from device import DEVICE

        device_str = str(DEVICE)
        if torch.cuda.is_available():
            gpu_name = torch.cuda.get_device_name(0)
            free, _total = torch.cuda.mem_get_info(0)
            vram_free_gb = round(free / (1024**3), 2)
        elif device_str == "mps":
            gpu_name = "Apple Silicon (MPS)"
        else:
            gpu_name = "CPU"
    except ImportError:
        device_str = "unknown"

    return {
        "status": "ok",
        "device": device_str,
        "gpu": gpu_name,
        "vram_free_gb": vram_free_gb,
    }


@app.get("/node_types")
def node_types() -> dict[str, Any]:
    """Return all registered node type definitions."""
    return registry.list_all()


def _make_serializable(obj: Any) -> Any:
    """Convert non-JSON-serializable objects to placeholder strings."""
    import math
    if isinstance(obj, dict):
        return {k: _make_serializable(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [_make_serializable(v) for v in obj]
    if isinstance(obj, float):
        # NaN/inf are not valid JSON (RFC 7159); serde_json will reject them.
        if math.isnan(obj) or math.isinf(obj):
            return None
        return obj
    if isinstance(obj, (str, int, bool, type(None))):
        return obj
    # torch tensors, models, etc. → placeholder
    return f"<{type(obj).__name__}>"


@app.post("/execute")
def execute(req: ExecuteRequest):
    """Execute a graph and return outputs."""
    log.info("POST /execute received")
    try:
        result = executor.execute(req.graph)
        log.info("POST /execute success")
        safe_result = _make_serializable(result)
        return JSONResponse(content=safe_result)
    except (KeyError, ValueError) as exc:
        log.error("POST /execute client error: %s", exc)
        raise HTTPException(status_code=400, detail=str(exc))
    except Exception as exc:
        log.exception("POST /execute server error")
        raise HTTPException(status_code=500, detail=str(exc))
