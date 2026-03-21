import logging
import time

import torch
from diffusers import (
    DDIMScheduler,
    EulerAncestralDiscreteScheduler,
    EulerDiscreteScheduler,
    HeunDiscreteScheduler,
    KDPM2AncestralDiscreteScheduler,
    KDPM2DiscreteScheduler,
    LMSDiscreteScheduler,
    UniPCMultistepScheduler,
)

from registry import NodeDef, PinDef, ParamDef

log = logging.getLogger("ksampler")

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

    # Create scheduler from the pipeline's original scheduler config
    # (attached by load_checkpoint), NOT from UNet config which lacks it.
    scheduler_cls = SCHEDULERS.get(sampler_name, EulerDiscreteScheduler)
    scheduler_config = getattr(unet, "_pipeline_scheduler_config", None)
    if scheduler_config:
        log.info("Using pipeline scheduler config: beta_start=%.5f, beta_end=%.4f, schedule=%s",
                 scheduler_config.get("beta_start", "?"),
                 scheduler_config.get("beta_end", "?"),
                 scheduler_config.get("beta_schedule", "?"))
        scheduler = scheduler_cls.from_config(scheduler_config)
    else:
        log.warning("No pipeline scheduler config found on UNet, using scheduler defaults!")
        scheduler = scheduler_cls()

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
        dtype=latent.dtype,
        device=unet.device,
    )

    # Sampling loop
    total_steps = len(timesteps)
    loop_start = time.time()

    for i, t in enumerate(timesteps):
        step_start = time.time()

        latent_input = torch.cat([latent] * 2)
        latent_input = scheduler.scale_model_input(latent_input, t)

        prompt_embeds = torch.cat([negative_embeds, positive_embeds])
        added_cond = {
            "text_embeds": torch.cat([negative_pooled, positive_pooled]),
            "time_ids": torch.cat([add_time_ids] * 2),
        }

        with torch.no_grad():
            noise_pred = unet(
                latent_input, t,
                encoder_hidden_states=prompt_embeds,
                added_cond_kwargs=added_cond,
            ).sample

        noise_pred_uncond, noise_pred_text = noise_pred.chunk(2)
        noise_pred = noise_pred_uncond + cfg * (noise_pred_text - noise_pred_uncond)

        latent = scheduler.step(noise_pred, t, latent).prev_sample

        step_time = time.time() - step_start
        elapsed = time.time() - loop_start
        avg = elapsed / (i + 1)
        eta = avg * (total_steps - i - 1)
        log.info(
            "Step %d/%d  %.2fs/step  elapsed %.1fs  ETA %.1fs",
            i + 1, total_steps, step_time, elapsed, eta,
        )

    log.info("Sampling done: %d steps in %.1fs", total_steps, time.time() - loop_start)
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
        ParamDef(
            "sampler_name", "ENUM", default="euler",
            options=[
                "euler", "euler_ancestral", "dpm_2", "dpm_2_ancestral",
                "lms", "heun", "ddim", "uni_pc",
            ],
        ),
        ParamDef(
            "scheduler", "ENUM", default="normal",
            options=["normal", "karras", "exponential", "sgm_uniform"],
        ),
    ],
    execute=execute,
)
