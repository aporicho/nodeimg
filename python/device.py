import torch


def get_device() -> torch.device:
    if torch.cuda.is_available():
        return torch.device("cuda")
    elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return torch.device("mps")
    else:
        return torch.device("cpu")


def get_dtype() -> torch.dtype:
    """MPS and CUDA support float16; CPU works better with float32."""
    dev = get_device()
    if dev.type == "cpu":
        return torch.float32
    return torch.float16


DEVICE = get_device()
DTYPE = get_dtype()
