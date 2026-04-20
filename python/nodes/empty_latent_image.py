import torch

from registry import Param, Pin, node
from device import DEVICE, DTYPE


@node(
    type_id="ai.empty_latent_image",
    title="Empty Latent Image",
    category="ai/latent",
    inputs=[],
    outputs=[Pin("latent", "LATENT")],
    params=[
        Param(
            "width", "INT", default=1024, expose=["control", "input"], min=64, max=4096
        ),
        Param(
            "height", "INT", default=1024, expose=["control", "input"], min=64, max=4096
        ),
        Param(
            "batch_size", "INT", default=1, expose=["control", "input"], min=1, max=16
        ),
    ],
)
def execute(ctx, inputs, params):
    w = params.get("width", 1024)
    h = params.get("height", 1024)
    batch = params.get("batch_size", 1)

    latent = torch.zeros(
        batch,
        4,
        h // 8,
        w // 8,
        dtype=DTYPE,
        device=DEVICE,
    )
    return {"latent": latent}
