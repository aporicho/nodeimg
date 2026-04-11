import logging

import torch
from diffusers import StableDiffusionXLPipeline

from registry import Param, Pin, node
from device import DEVICE, DTYPE

log = logging.getLogger("load_checkpoint")

# SDXL VAE uses 0.13025; SD1.5 uses 0.18215.
# from_single_file often misdetects this for SDXL checkpoints.
SDXL_VAE_SCALING_FACTOR = 0.13025


@node(
    type_id="ai.load_checkpoint",
    title="Load Checkpoint",
    category="ai/model",
    inputs=[],
    outputs=[
        Pin("model", "MODEL"),
        Pin("clip", "CLIP"),
        Pin("vae", "VAE"),
    ],
    params=[
        Param(
            "checkpoint_path",
            "STRING",
            default=None,
            expose=["control"],
            widget="file_picker",
        ),
    ],
)
def execute(ctx, inputs, params):
    path = params["checkpoint_path"]
    log.info("Loading checkpoint: %s", path)
    log.info("Device: %s, dtype: %s", DEVICE, DTYPE)

    pipe = StableDiffusionXLPipeline.from_single_file(
        path,
        torch_dtype=DTYPE,
        use_safetensors=True,
    )
    pipe.to(DEVICE)

    # Fix scaling_factor: from_single_file often loads SDXL with the wrong value
    original_sf = pipe.vae.config.scaling_factor
    if abs(original_sf - SDXL_VAE_SCALING_FACTOR) > 1e-4:
        log.warning(
            "VAE scaling_factor is %.5f, expected %.5f for SDXL — overriding",
            original_sf,
            SDXL_VAE_SCALING_FACTOR,
        )
        pipe.vae.config.scaling_factor = SDXL_VAE_SCALING_FACTOR
    else:
        log.info("VAE scaling_factor: %.5f (correct)", original_sf)

    log.info("UNet in_channels: %d", pipe.unet.config.in_channels)
    log.info("VAE latent_channels: %d", pipe.vae.config.latent_channels)

    # Attach the pipeline's scheduler config to the UNet so KSampler can use it.
    # The UNet config doesn't contain scheduler params, but KSampler needs the
    # correct beta schedule for proper noise prediction.
    sched_cfg = dict(pipe.scheduler.config)
    pipe.unet._pipeline_scheduler_config = sched_cfg
    log.info(
        "Scheduler config: beta_start=%.5f, beta_end=%.4f, schedule=%s, prediction=%s",
        sched_cfg.get("beta_start", "?"),
        sched_cfg.get("beta_end", "?"),
        sched_cfg.get("beta_schedule", "?"),
        sched_cfg.get("prediction_type", "?"),
    )

    result = {
        "model": pipe.unet,
        "clip": (
            pipe.tokenizer,
            pipe.text_encoder,
            pipe.tokenizer_2,
            pipe.text_encoder_2,
        ),
        "vae": pipe.vae,
    }

    # Free the pipeline wrapper, keep components
    del pipe
    if torch.cuda.is_available():
        torch.cuda.empty_cache()

    log.info("Checkpoint loaded successfully")
    return result
