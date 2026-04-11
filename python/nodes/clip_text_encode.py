import torch

from registry import Param, Pin, node


@node(
    type_id="ai.clip_text_encode",
    title="CLIP Text Encode",
    category="ai/conditioning",
    inputs=[Pin("clip", "CLIP", required=True)],
    outputs=[Pin("conditioning", "CONDITIONING")],
    params=[Param("text", "STRING", default="", expose=["control", "input"])],
)
def execute(ctx, inputs, params):
    tokenizer, text_encoder, tokenizer_2, text_encoder_2 = inputs["clip"]
    text = params.get("text", "")

    # Encode with first text encoder
    tokens_1 = tokenizer(
        text,
        padding="max_length",
        max_length=77,
        truncation=True,
        return_tensors="pt",
    ).input_ids.to(text_encoder.device)
    with torch.no_grad():
        enc_1 = text_encoder(tokens_1, output_hidden_states=True)
    hidden_1 = enc_1.hidden_states[-2]

    # Encode with second text encoder
    tokens_2 = tokenizer_2(
        text,
        padding="max_length",
        max_length=77,
        truncation=True,
        return_tensors="pt",
    ).input_ids.to(text_encoder_2.device)
    with torch.no_grad():
        enc_2 = text_encoder_2(tokens_2, output_hidden_states=True)
    hidden_2 = enc_2.hidden_states[-2]
    pooled = enc_2[0]

    prompt_embeds = torch.cat([hidden_1, hidden_2], dim=-1)

    return {"conditioning": (prompt_embeds, pooled)}
