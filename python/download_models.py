#!/usr/bin/env python3
"""从魔塔(ModelScope)下载 SDXL 所需模型到 models/ 目录。

使用方法:
    python download_models.py              # 下载所有模型
    python download_models.py --list       # 查看可下载的模型列表
    python download_models.py sdxl-base    # 只下载指定模型

需要先安装: pip install modelscope
"""

import argparse
import os
import sys
from pathlib import Path

# 模型目录（项目根目录下的 models/）
MODELS_DIR = Path(__file__).parent.parent / "models"

# 可下载的模型定义
MODELS = {
    "sdxl-base": {
        "model_id": "AI-ModelScope/stable-diffusion-xl-base-1.0",
        "description": "Stable Diffusion XL Base 1.0 (单文件 checkpoint，~6.5GB)",
        "subdir": "stable-diffusion-xl-base-1.0",
        "allow_patterns": ["sd_xl_base_1.0.safetensors"],
    },
}


def check_modelscope():
    """检查 modelscope 是否已安装。"""
    try:
        import modelscope  # noqa: F401
        return True
    except ImportError:
        print("错误: 未安装 modelscope SDK")
        print("请运行: pip install modelscope")
        return False


def list_models():
    """列出所有可下载的模型。"""
    print("可下载的模型:\n")
    for key, info in MODELS.items():
        target = MODELS_DIR / info["subdir"]
        status = "✓ 已下载" if target.exists() else "✗ 未下载"
        print(f"  {key:20s} {status}")
        print(f"  {'':20s} {info['description']}")
        print(f"  {'':20s} 来源: modelscope.cn/models/{info['model_id']}")
        print()


def download_model(key: str, force: bool = False):
    """下载指定模型。支持断点续传，已完整的文件会自动跳过。"""
    if key not in MODELS:
        print(f"错误: 未知模型 '{key}'")
        print(f"可用模型: {', '.join(MODELS.keys())}")
        return False

    info = MODELS[key]
    target = MODELS_DIR / info["subdir"]

    if force and target.exists():
        import shutil
        print(f"强制模式: 删除已有目录 {target}")
        shutil.rmtree(target)

    print(f"正在同步: {info['description']}")
    print(f"来源: modelscope.cn/models/{info['model_id']}")
    print(f"目标: {target}")
    print()

    from modelscope import snapshot_download

    MODELS_DIR.mkdir(parents=True, exist_ok=True)

    kwargs = {}
    if "allow_patterns" in info:
        kwargs["allow_patterns"] = info["allow_patterns"]

    snapshot_download(
        model_id=info["model_id"],
        local_dir=str(target),
        **kwargs,
    )

    print(f"\n同步完成: {target}")
    return True


def main():
    parser = argparse.ArgumentParser(description="下载 SDXL 所需模型（从魔塔 ModelScope）")
    parser.add_argument("model", nargs="?", default=None,
                        help=f"要下载的模型名称（可选: {', '.join(MODELS.keys())}）")
    parser.add_argument("--list", action="store_true", help="列出所有可下载的模型")
    parser.add_argument("--all", action="store_true", help="下载所有模型")
    parser.add_argument("--force", action="store_true", help="强制重新下载（删除已有文件）")

    args = parser.parse_args()

    if args.list:
        list_models()
        return

    if not check_modelscope():
        sys.exit(1)

    if args.all:
        for key in MODELS:
            download_model(key, force=args.force)
        return

    if args.model:
        if not download_model(args.model, force=args.force):
            sys.exit(1)
        return

    # 默认: 下载所有模型
    print("将下载所有 SDXL 所需模型...\n")
    for key in MODELS:
        download_model(key, force=args.force)


if __name__ == "__main__":
    main()
