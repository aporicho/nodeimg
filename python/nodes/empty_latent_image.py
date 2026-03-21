import torch

from registry import NodeDef, PinDef, ParamDef
from device import DEVICE, DTYPE


def execute(inputs, params):
    w = params.get("width", 1024)
    h = params.get("height", 1024)
    batch = params.get("batch_size", 1)

    latent = torch.zeros(
        batch, 4, h // 8, w // 8,
        dtype=DTYPE, device=DEVICE,
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
