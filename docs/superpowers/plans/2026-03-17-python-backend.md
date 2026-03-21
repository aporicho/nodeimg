# Python Backend Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Python FastAPI backend that receives SDXL node graphs via HTTP and executes them using diffusers/torch.

**Architecture:** NodeRegistry holds node type definitions, GraphExecutor performs topological sort and sequential execution, Server exposes HTTP endpoints. One POST /execute receives the full graph, one GET /node_types returns definitions for frontend dynamic registration.

**Tech Stack:** Python 3.10+, FastAPI, uvicorn, torch, diffusers, transformers

**Spec:** `docs/superpowers/specs/2026-03-17-sdxl-nodes-design.md`
**Architecture doc:** `docs/backend-architecture.md`
**Protocol doc:** `docs/protocol.md`

---

## Chunk 1: Core Framework

### Task 1: Project setup and NodeRegistry

**Files:**
- Create: `python/requirements.txt`
- Create: `python/registry.py`
- Create: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/requirements.txt`**

```
torch
diffusers
transformers
safetensors
accelerate
fastapi
uvicorn
```

- [ ] **Step 2: Create `python/registry.py`**

```python
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional


@dataclass
class PinDef:
    name: str
    type: str


@dataclass
class ParamDef:
    name: str
    type: str  # STRING, INT, FLOAT, ENUM
    default: Any = None
    min: Optional[float] = None
    max: Optional[float] = None
    options: Optional[List[str]] = None
    widget: Optional[str] = None


@dataclass
class NodeDef:
    inputs: List[PinDef] = field(default_factory=list)
    outputs: List[PinDef] = field(default_factory=list)
    params: List[ParamDef] = field(default_factory=list)
    execute: Callable = None


class NodeRegistry:
    def __init__(self):
        self._nodes: Dict[str, NodeDef] = {}

    def register(self, node_type: str, definition: NodeDef):
        self._nodes[node_type] = definition

    def get(self, node_type: str) -> NodeDef:
        if node_type not in self._nodes:
            raise KeyError(f"Unknown node type: {node_type}")
        return self._nodes[node_type]

    def list_all(self) -> Dict[str, dict]:
        result = {}
        for ntype, ndef in self._nodes.items():
            result[ntype] = {
                "inputs": [{"name": p.name, "type": p.type} for p in ndef.inputs],
                "outputs": [{"name": p.name, "type": p.type} for p in ndef.outputs],
                "params": [_param_to_dict(p) for p in ndef.params],
            }
        return result


def _param_to_dict(p: ParamDef) -> dict:
    d = {"name": p.name, "type": p.type}
    if p.default is not None:
        d["default"] = p.default
    if p.min is not None:
        d["min"] = p.min
    if p.max is not None:
        d["max"] = p.max
    if p.options is not None:
        d["options"] = p.options
    if p.widget is not None:
        d["widget"] = p.widget
    return d
```

- [ ] **Step 3: Create empty `python/nodes/__init__.py`**

```python
from python.registry import NodeRegistry


def register_all(registry: NodeRegistry):
    """Register all built-in nodes. Called by server on startup."""
    pass  # Nodes added in later tasks
```

- [ ] **Step 4: Verify imports work**

Run: `cd python && python -c "from registry import NodeRegistry; r = NodeRegistry(); print('OK')"`
Expected: `OK`

- [ ] **Step 5: Commit**

```bash
git add python/
git commit -m "feat(backend): add NodeRegistry and project setup"
```

---

### Task 2: GraphExecutor

**Files:**
- Create: `python/executor.py`

- [ ] **Step 1: Create `python/executor.py`**

```python
from collections import defaultdict, deque
from typing import Any, Dict, List

from registry import NodeRegistry


class GraphExecutor:
    def __init__(self, registry: NodeRegistry):
        self.registry = registry

    def execute(self, graph: dict) -> dict:
        nodes = graph["nodes"]
        connections = graph["connections"]
        output_node = graph["output_node"]

        order = self._topo_sort(nodes, connections)

        # Execute nodes in topological order
        results: Dict[str, Dict[str, Any]] = {}
        for node_id in order:
            node = nodes[node_id]
            node_def = self.registry.get(node["type"])

            # Gather inputs from upstream results
            inputs = {}
            for conn in connections:
                if conn["to_node"] == node_id:
                    from_id = conn["from_node"]
                    from_output = conn["from_output"]
                    to_input = conn["to_input"]
                    inputs[to_input] = results[from_id][from_output]

            # Merge params
            params = node.get("params", {})

            # Execute
            outputs = node_def.execute(inputs, params)
            results[node_id] = outputs

        # Return output node results
        return {"outputs": {output_node: results.get(output_node, {})}}

    def _topo_sort(self, nodes: dict, connections: list) -> List[str]:
        in_degree = defaultdict(int)
        adjacency = defaultdict(list)
        all_nodes = set(nodes.keys())

        for node_id in all_nodes:
            in_degree[node_id] = 0

        for conn in connections:
            to_id = conn["to_node"]
            from_id = conn["from_node"]
            adjacency[from_id].append(to_id)
            in_degree[to_id] += 1

        queue = deque([n for n in all_nodes if in_degree[n] == 0])
        order = []

        while queue:
            node_id = queue.popleft()
            order.append(node_id)
            for neighbor in adjacency[node_id]:
                in_degree[neighbor] -= 1
                if in_degree[neighbor] == 0:
                    queue.append(neighbor)

        if len(order) != len(all_nodes):
            raise ValueError("Graph has cycle")

        return order
```

- [ ] **Step 2: Test with a mock graph**

Run:
```bash
cd python && python -c "
from registry import NodeRegistry, NodeDef, PinDef
from executor import GraphExecutor

reg = NodeRegistry()
reg.register('Add', NodeDef(
    inputs=[PinDef('a', 'INT'), PinDef('b', 'INT')],
    outputs=[PinDef('result', 'INT')],
    execute=lambda inputs, params: {'result': inputs['a'] + inputs['b']}
))
reg.register('Const', NodeDef(
    outputs=[PinDef('value', 'INT')],
    params=[],
    execute=lambda inputs, params: {'value': params.get('value', 0)}
))

graph = {
    'nodes': {
        '1': {'type': 'Const', 'params': {'value': 3}},
        '2': {'type': 'Const', 'params': {'value': 5}},
        '3': {'type': 'Add', 'params': {}},
    },
    'connections': [
        {'from_node': '1', 'from_output': 'value', 'to_node': '3', 'to_input': 'a'},
        {'from_node': '2', 'from_output': 'value', 'to_node': '3', 'to_input': 'b'},
    ],
    'output_node': '3'
}

ex = GraphExecutor(reg)
result = ex.execute(graph)
assert result['outputs']['3']['result'] == 8
print('OK: 3 + 5 =', result['outputs']['3']['result'])
"
```
Expected: `OK: 3 + 5 = 8`

- [ ] **Step 3: Commit**

```bash
git add python/executor.py
git commit -m "feat(backend): add GraphExecutor with topological sort"
```

---

### Task 3: FastAPI Server

**Files:**
- Create: `python/server.py`

- [ ] **Step 1: Create `python/server.py`**

```python
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Any, Dict

from registry import NodeRegistry
from executor import GraphExecutor
from nodes import register_all

app = FastAPI(title="Node Image Studio Backend")
registry = NodeRegistry()
executor = GraphExecutor(registry)

register_all(registry)


class ExecuteRequest(BaseModel):
    graph: Dict[str, Any]


@app.get("/health")
def health():
    import torch
    gpu = "none"
    vram_free_gb = 0.0
    if torch.cuda.is_available():
        gpu = torch.cuda.get_device_name(0)
        free, total = torch.cuda.mem_get_info(0)
        vram_free_gb = round(free / (1024 ** 3), 1)
    return {"status": "ok", "gpu": gpu, "vram_free_gb": vram_free_gb}


@app.get("/node_types")
def node_types():
    return {"node_types": registry.list_all()}


@app.post("/execute")
def execute(req: ExecuteRequest):
    try:
        result = executor.execute(req.graph)
        return result
    except KeyError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))
```

- [ ] **Step 2: Test server starts**

Run: `cd python && timeout 5 uvicorn server:app --port 8188 || true`
Expected: Server starts without import errors (will timeout after 5s, that's fine).

- [ ] **Step 3: Commit**

```bash
git add python/server.py
git commit -m "feat(backend): add FastAPI server with /execute, /health, /node_types"
```

---

## Chunk 2: SDXL Nodes

### Task 4: LoadCheckpoint node

**Files:**
- Create: `python/nodes/load_checkpoint.py`
- Modify: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/nodes/load_checkpoint.py`**

```python
import torch
from diffusers import StableDiffusionXLPipeline

from registry import NodeDef, PinDef, ParamDef


def execute(inputs, params):
    path = params["checkpoint_path"]
    pipe = StableDiffusionXLPipeline.from_single_file(
        path,
        torch_dtype=torch.float16,
        use_safetensors=True,
    )
    pipe.to("cuda")

    result = {
        "model": pipe.unet,
        "clip": (pipe.tokenizer, pipe.text_encoder, pipe.tokenizer_2, pipe.text_encoder_2),
        "vae": pipe.vae,
    }

    # Free the pipeline wrapper, keep components
    del pipe
    torch.cuda.empty_cache()

    return result


definition = NodeDef(
    inputs=[],
    outputs=[
        PinDef("model", "MODEL"),
        PinDef("clip", "CLIP"),
        PinDef("vae", "VAE"),
    ],
    params=[
        ParamDef("checkpoint_path", "STRING", widget="file_picker"),
    ],
    execute=execute,
)
```

- [ ] **Step 2: Register in `python/nodes/__init__.py`**

```python
from registry import NodeRegistry


def register_all(registry: NodeRegistry):
    from nodes.load_checkpoint import definition as load_checkpoint
    registry.register("LoadCheckpoint", load_checkpoint)
```

- [ ] **Step 3: Commit**

```bash
git add python/nodes/
git commit -m "feat(backend): add LoadCheckpoint node"
```

---

### Task 5: CLIPTextEncode node

**Files:**
- Create: `python/nodes/clip_text_encode.py`
- Modify: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/nodes/clip_text_encode.py`**

```python
import torch

from registry import NodeDef, PinDef, ParamDef


def execute(inputs, params):
    tokenizer, text_encoder, tokenizer_2, text_encoder_2 = inputs["clip"]
    text = params.get("text", "")

    # Encode with first text encoder
    tokens_1 = tokenizer(text, padding="max_length", max_length=77,
                          truncation=True, return_tensors="pt").input_ids.to(text_encoder.device)
    with torch.no_grad():
        enc_1 = text_encoder(tokens_1, output_hidden_states=True)
    hidden_1 = enc_1.hidden_states[-2]

    # Encode with second text encoder
    tokens_2 = tokenizer_2(text, padding="max_length", max_length=77,
                            truncation=True, return_tensors="pt").input_ids.to(text_encoder_2.device)
    with torch.no_grad():
        enc_2 = text_encoder_2(tokens_2, output_hidden_states=True)
    hidden_2 = enc_2.hidden_states[-2]
    pooled = enc_2[0]

    prompt_embeds = torch.cat([hidden_1, hidden_2], dim=-1)

    return {"conditioning": (prompt_embeds, pooled)}


definition = NodeDef(
    inputs=[PinDef("clip", "CLIP")],
    outputs=[PinDef("conditioning", "CONDITIONING")],
    params=[ParamDef("text", "STRING", default="")],
    execute=execute,
)
```

- [ ] **Step 2: Register in `__init__.py`**

Add to `register_all`:
```python
from nodes.clip_text_encode import definition as clip_text_encode
registry.register("CLIPTextEncode", clip_text_encode)
```

- [ ] **Step 3: Commit**

```bash
git add python/nodes/
git commit -m "feat(backend): add CLIPTextEncode node"
```

---

### Task 6: EmptyLatentImage node

**Files:**
- Create: `python/nodes/empty_latent_image.py`
- Modify: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/nodes/empty_latent_image.py`**

```python
import torch

from registry import NodeDef, PinDef, ParamDef


def execute(inputs, params):
    width = params.get("width", 1024)
    height = params.get("height", 1024)
    batch_size = params.get("batch_size", 1)

    latent = torch.zeros(
        batch_size, 4, height // 8, width // 8,
        dtype=torch.float16, device="cuda"
    )
    return {"latent": latent}


definition = NodeDef(
    inputs=[],
    outputs=[PinDef("latent", "LATENT")],
    params=[
        ParamDef("width", "INT", default=1024, min=64, max=4096),
        ParamDef("height", "INT", default=1024, min=64, max=4096),
        ParamDef("batch_size", "INT", default=1, min=1, max=16),
    ],
    execute=execute,
)
```

- [ ] **Step 2: Register in `__init__.py`**

Add to `register_all`:
```python
from nodes.empty_latent_image import definition as empty_latent
registry.register("EmptyLatentImage", empty_latent)
```

- [ ] **Step 3: Commit**

```bash
git add python/nodes/
git commit -m "feat(backend): add EmptyLatentImage node"
```

---

### Task 7: KSampler node

**Files:**
- Create: `python/nodes/ksampler.py`
- Modify: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/nodes/ksampler.py`**

```python
import torch
from diffusers import (
    EulerDiscreteScheduler,
    EulerAncestralDiscreteScheduler,
    KDPM2DiscreteScheduler,
    KDPM2AncestralDiscreteScheduler,
    LMSDiscreteScheduler,
    HeunDiscreteScheduler,
    DDIMScheduler,
    UniPCMultistepScheduler,
)

from registry import NodeDef, PinDef, ParamDef

SCHEDULERS = {
    "euler": EulerDiscreteScheduler,
    "euler_ancestral": EulerAncestralDiscreteScheduler,
    "dpm_2": KDPM2DiscreteScheduler,
    "dpm_2_ancestral": KDPM2AncestralDiscreteScheduler,
    "lms": LMSDiscreteScheduler,
    "heun": HeunDiscreteScheduler,
    "ddim": DDIMScheduler,
    "uni_pc": UniPCMultistepScheduler,
}


def execute(inputs, params):
    unet = inputs["model"]
    positive_embeds, positive_pooled = inputs["positive"]
    negative_embeds, negative_pooled = inputs["negative"]
    latent = inputs["latent"]

    seed = params.get("seed", 0)
    steps = params.get("steps", 20)
    cfg = params.get("cfg", 7.0)
    sampler_name = params.get("sampler_name", "euler")
    scheduler_type = params.get("scheduler", "normal")

    # Create scheduler
    scheduler_cls = SCHEDULERS.get(sampler_name, EulerDiscreteScheduler)
    scheduler_config = dict(unet.config.get("scheduler_config", {})) if hasattr(unet.config, "get") else {}
    scheduler = scheduler_cls.from_config(scheduler_config) if scheduler_config else scheduler_cls()

    if scheduler_type == "karras":
        scheduler.config.use_karras_sigmas = True

    scheduler.set_timesteps(steps, device=unet.device)
    timesteps = scheduler.timesteps

    # Initialize noise
    generator = torch.Generator(device=unet.device).manual_seed(seed)
    noise = torch.randn_like(latent, generator=generator)
    latent = scheduler.add_noise(latent, noise, timesteps[:1])

    # Build added_cond_kwargs for SDXL
    add_text_embeds = positive_pooled
    add_time_ids = torch.tensor(
        [[1024.0, 1024.0, 0.0, 0.0, 1024.0, 1024.0]],
        dtype=torch.float16, device=unet.device
    )

    # Sampling loop
    for t in timesteps:
        latent_input = torch.cat([latent] * 2)
        latent_input = scheduler.scale_model_input(latent_input, t)

        prompt_embeds = torch.cat([negative_embeds, positive_embeds])
        added_cond = {
            "text_embeds": torch.cat([negative_pooled, positive_pooled]),
            "time_ids": torch.cat([add_time_ids] * 2),
        }

        with torch.no_grad():
            noise_pred = unet(latent_input, t, encoder_hidden_states=prompt_embeds,
                              added_cond_kwargs=added_cond).sample

        noise_pred_uncond, noise_pred_text = noise_pred.chunk(2)
        noise_pred = noise_pred_uncond + cfg * (noise_pred_text - noise_pred_uncond)

        latent = scheduler.step(noise_pred, t, latent).prev_sample

    return {"latent": latent}


definition = NodeDef(
    inputs=[
        PinDef("model", "MODEL"),
        PinDef("positive", "CONDITIONING"),
        PinDef("negative", "CONDITIONING"),
        PinDef("latent", "LATENT"),
    ],
    outputs=[PinDef("latent", "LATENT")],
    params=[
        ParamDef("seed", "INT", default=0, min=0, max=2147483647),
        ParamDef("steps", "INT", default=20, min=1, max=150),
        ParamDef("cfg", "FLOAT", default=7.0, min=1.0, max=30.0),
        ParamDef("sampler_name", "ENUM", default="euler",
                 options=["euler", "euler_ancestral", "dpm_2", "dpm_2_ancestral",
                          "lms", "heun", "ddim", "uni_pc"]),
        ParamDef("scheduler", "ENUM", default="normal",
                 options=["normal", "karras", "exponential", "sgm_uniform"]),
    ],
    execute=execute,
)
```

- [ ] **Step 2: Register in `__init__.py`**

Add to `register_all`:
```python
from nodes.ksampler import definition as ksampler
registry.register("KSampler", ksampler)
```

- [ ] **Step 3: Commit**

```bash
git add python/nodes/
git commit -m "feat(backend): add KSampler node with scheduler support"
```

---

### Task 8: VAEDecode node

**Files:**
- Create: `python/nodes/vae_decode.py`
- Modify: `python/nodes/__init__.py`

- [ ] **Step 1: Create `python/nodes/vae_decode.py`**

```python
import base64
import io

import torch
from PIL import Image as PILImage

from registry import NodeDef, PinDef


def execute(inputs, params):
    vae = inputs["vae"]
    latent = inputs["latent"]

    # Decode
    with torch.no_grad():
        latent = latent / vae.config.scaling_factor
        image_tensor = vae.decode(latent).sample

    # Tensor → PIL → base64 PNG
    image_tensor = (image_tensor / 2 + 0.5).clamp(0, 1)
    image_np = image_tensor[0].cpu().permute(1, 2, 0).float().numpy()
    image_np = (image_np * 255).round().astype("uint8")
    pil_image = PILImage.fromarray(image_np)

    buffer = io.BytesIO()
    pil_image.save(buffer, format="PNG")
    b64 = base64.b64encode(buffer.getvalue()).decode("utf-8")

    return {"image": b64}


definition = NodeDef(
    inputs=[
        PinDef("vae", "VAE"),
        PinDef("latent", "LATENT"),
    ],
    outputs=[PinDef("image", "IMAGE")],
    params=[],
    execute=execute,
)
```

- [ ] **Step 2: Register in `__init__.py`**

Final `__init__.py`:
```python
from registry import NodeRegistry


def register_all(registry: NodeRegistry):
    from nodes.load_checkpoint import definition as load_checkpoint
    from nodes.clip_text_encode import definition as clip_text_encode
    from nodes.empty_latent_image import definition as empty_latent
    from nodes.ksampler import definition as ksampler
    from nodes.vae_decode import definition as vae_decode

    registry.register("LoadCheckpoint", load_checkpoint)
    registry.register("CLIPTextEncode", clip_text_encode)
    registry.register("EmptyLatentImage", empty_latent)
    registry.register("KSampler", ksampler)
    registry.register("VAEDecode", vae_decode)
```

- [ ] **Step 3: Test server starts with all nodes**

Run: `cd python && timeout 5 uvicorn server:app --port 8188 || true`
Expected: No import errors.

- [ ] **Step 4: Test /node_types returns all 5 nodes**

Run:
```bash
cd python && uvicorn server:app --port 8188 &
sleep 2
curl -s http://localhost:8188/node_types | python -m json.tool | head -30
kill %1
```
Expected: JSON with LoadCheckpoint, CLIPTextEncode, EmptyLatentImage, KSampler, VAEDecode.

- [ ] **Step 5: Commit**

```bash
git add python/
git commit -m "feat(backend): add VAEDecode node, complete SDXL pipeline"
```

---

## Chunk 3: End-to-End Test

### Task 9: Manual integration test

This requires an SDXL model file on disk and a CUDA GPU.

- [ ] **Step 1: Start the server**

Run: `cd python && uvicorn server:app --host 0.0.0.0 --port 8188`

- [ ] **Step 2: Test /health**

Run: `curl http://localhost:8188/health`
Expected: `{"status":"ok","gpu":"NVIDIA ...","vram_free_gb":...}`

- [ ] **Step 3: Test /execute with full SDXL pipeline**

Run:
```bash
curl -X POST http://localhost:8188/execute \
  -H "Content-Type: application/json" \
  -d '{
    "graph": {
      "nodes": {
        "1": {"type": "LoadCheckpoint", "params": {"checkpoint_path": "/path/to/sd_xl_base_1.0.safetensors"}},
        "2": {"type": "CLIPTextEncode", "params": {"text": "a photo of a cat"}},
        "3": {"type": "CLIPTextEncode", "params": {"text": "ugly, blurry"}},
        "4": {"type": "EmptyLatentImage", "params": {"width": 1024, "height": 1024, "batch_size": 1}},
        "5": {"type": "KSampler", "params": {"seed": 42, "steps": 20, "cfg": 7.0, "sampler_name": "euler", "scheduler": "normal"}},
        "6": {"type": "VAEDecode", "params": {}}
      },
      "connections": [
        {"from_node": "1", "from_output": "model", "to_node": "5", "to_input": "model"},
        {"from_node": "1", "from_output": "clip", "to_node": "2", "to_input": "clip"},
        {"from_node": "1", "from_output": "clip", "to_node": "3", "to_input": "clip"},
        {"from_node": "1", "from_output": "vae", "to_node": "6", "to_input": "vae"},
        {"from_node": "2", "from_output": "conditioning", "to_node": "5", "to_input": "positive"},
        {"from_node": "3", "from_output": "conditioning", "to_node": "5", "to_input": "negative"},
        {"from_node": "4", "from_output": "latent", "to_node": "5", "to_input": "latent"},
        {"from_node": "5", "from_output": "latent", "to_node": "6", "to_input": "latent"}
      ],
      "output_node": "6"
    }
  }' | python -c "
import sys, json, base64
resp = json.load(sys.stdin)
img_b64 = resp['outputs']['6']['image']
with open('/tmp/test_output.png', 'wb') as f:
    f.write(base64.b64decode(img_b64))
print('Image saved to /tmp/test_output.png')
print('Size:', len(base64.b64decode(img_b64)), 'bytes')
"
```
Expected: `Image saved to /tmp/test_output.png` with a valid PNG file.

- [ ] **Step 4: Verify the image**

Open `/tmp/test_output.png` and verify it's a 1024x1024 image of a cat.

- [ ] **Step 5: Commit final state**

```bash
git add -A
git commit -m "feat(backend): complete SDXL pipeline, tested end-to-end"
```
