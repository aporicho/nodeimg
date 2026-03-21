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
