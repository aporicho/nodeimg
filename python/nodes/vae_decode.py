import base64
import io
import logging

import torch
from PIL import Image as PILImage

from registry import Pin, node

log = logging.getLogger("vae_decode")


@node(
    type_id="ai.vae_decode",
    title="VAE Decode",
    category="ai/image",
    inputs=[
        Pin("vae", "VAE", required=True),
        Pin("latent", "LATENT", required=True),
    ],
    outputs=[Pin("image", "IMAGE")],
    params=[],
)
def execute(ctx, inputs, params):
    vae = inputs["vae"]
    latent = inputs["latent"]

    log.info("Latent shape: %s, dtype: %s", latent.shape, latent.dtype)
    log.info(
        "Latent stats — min: %.4f, max: %.4f, mean: %.4f",
        latent.min().item(),
        latent.max().item(),
        latent.mean().item(),
    )
    log.info("VAE scaling_factor: %.5f", vae.config.scaling_factor)

    # SDXL VAE is numerically unstable in float16 — produces NaN values.
    # Temporarily upcast VAE to float32 for decoding, then restore.
    # See: https://huggingface.co/madebyollin/sdxl-vae-fp16-fix
    original_dtype = vae.dtype
    vae.to(dtype=torch.float32)

    with torch.no_grad():
        scaled_latent = latent.to(dtype=torch.float32) / vae.config.scaling_factor
        log.info(
            "Scaled latent stats — min: %.4f, max: %.4f, mean: %.4f",
            scaled_latent.min().item(),
            scaled_latent.max().item(),
            scaled_latent.mean().item(),
        )
        image_tensor = vae.decode(scaled_latent).sample

    vae.to(dtype=original_dtype)

    log.info(
        "Decoded tensor stats — min: %.4f, max: %.4f, mean: %.4f",
        image_tensor.min().item(),
        image_tensor.max().item(),
        image_tensor.mean().item(),
    )

    # Check for NaN — indicates VAE fp16 instability
    if torch.isnan(image_tensor).any():
        log.error("NaN detected in decoded image! VAE may need fp16-fix variant.")

    # Tensor -> PIL -> base64 PNG
    image_tensor = (image_tensor / 2 + 0.5).clamp(0, 1)
    image_np = image_tensor[0].cpu().permute(1, 2, 0).float().numpy()
    image_np = (image_np * 255).round().astype("uint8")
    pil_image = PILImage.fromarray(image_np)

    log.info("Output image size: %s, mode: %s", pil_image.size, pil_image.mode)

    buffer = io.BytesIO()
    pil_image.save(buffer, format="PNG")
    b64 = base64.b64encode(buffer.getvalue()).decode("utf-8")

    return {"image": b64}
