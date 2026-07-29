#!/usr/bin/env python3
"""
LoRA Fine-Tuning Studio
Requirements: pip install transformers peft trl bitsandbytes accelerate gradio datasets torch
"""
import os
import json
import threading
import time
from dataclasses import asdict
from typing import Optional, List

import gradio as gr
import torch
from datasets import load_dataset
from peft import LoraConfig, get_peft_model, PeftModel
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    BitsAndBytesConfig,
    TrainingArguments,
)
from trl import SFTTrainer, DataCollatorForCompletionOnlyLM

# ── Globals ──────────────────────────────────────────────────────────
MODEL_REGISTRY = {
    "meta-llama/Llama-2-7b-chat-hf": {"params": "7B", "arch": "llama"},
    "meta-llama/Meta-Llama-3-8B-Instruct": {"params": "8B", "arch": "llama"},
    "mistralai/Mistral-7B-Instruct-v0.2": {"params": "7B", "arch": "mistral"},
    "Qwen/Qwen2-7B-Instruct": {"params": "7B", "arch": "qwen"},
    "microsoft/Phi-3-mini-4k-instruct": {"params": "3.8B", "arch": "phi"},
    "google/gemma-2-2b-it": {"params": "2B", "arch": "gemma"},
}

DATASET_REGISTRY = {
    "tatsu-lab/alpaca": {"rows": "~52K", "format": "alpaca"},
    "databricks/databricks-dolly-15k": {"rows": "~15K", "format": "dolly"},
    "OpenAssistant/oasst_top1_2023-08-25": {"rows": "~12K", "format": "oasst"},
    "HuggingFaceH4/no_robots": {"rows": "~9.5K", "format": "sharegpt"},
    "iamtarun/python_code_instructions_18k_alpaca": {"rows": "~18K", "format": "alpaca"},
}

DEFAULT_LORA_TARGETS = {
    "llama": ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    "mistral": ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    "qwen": ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    "phi": ["q_proj", "k_proj", "v_proj", "dense", "fc1", "fc2"],
    "gemma": ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
}

# Shared training state
_training_state = {
    "trainer": None,
    "logs": [],
    "metrics": [],
    "cancelled": False,
    "model": None,
    "tokenizer": None,
    "adapter_path": None,
}


# ── Helpers ──────────────────────────────────────────────────────────
def estimate_vram(model_id: str, quant: str, lora_r: int, max_seq: int, batch: int) -> dict:
    """Rough VRAM estimator (GB)."""
    params = MODEL_REGISTRY.get(model_id, {}).get("params", "7B")
    base = {"2B": 2.5, "3.8B": 4.0, "7B": 7.0, "8B": 8.0, "13B": 13.0}.get(params, 7.0)
    q_mult = {"4-bit": 0.55, "8-bit": 0.70, "FP16": 1.0, "BF16": 1.0}.get(quant, 1.0)
    lora_overhead = (lora_r / 64) * 0.5  # rough
    seq_overhead = (max_seq / 2048) * 1.5
    batch_overhead = batch * 0.4
    total = base * q_mult + lora_overhead + seq_overhead + batch_overhead + 1.5  # activations
    return {
        "total_gb": round(total, 1),
        "base_gb": round(base * q_mult, 1),
        "overhead_gb": round(lora_overhead + seq_overhead + batch_overhead + 1.5, 1),
        "safe": total <= 24.0,
    }


def build_bnb_config(quant: str):
    if quant == "4-bit":
        return BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_compute_dtype=torch.bfloat16,
            bnb_4bit_use_double_quant=True,
            bnb_4bit_quant_type="nf4",
        )
    elif quant == "8-bit":
        return BitsAndBytesConfig(load_in_8bit=True)
    return None


def format_dataset(examples, format_type: str, tokenizer):
    """Format dataset examples into text."""
    texts = []
    for i in range(len(examples.get("instruction", examples.get("prompt", [])))):
        if format_type == "alpaca":
            inst = examples.get("instruction", [""])[i]
            inp = examples.get("input", [""])[i]
            out = examples.get("output", [""])[i]
            if inp:
                text = f"### Instruction:\n{inst}\n\n### Input:\n{inp}\n\n### Response:\n{out}"
            else:
                text = f"### Instruction:\n{inst}\n\n### Response:\n{out}"
        elif format_type == "dolly":
            inst = examples.get("instruction", [""])[i]
            ctx = examples.get("context", [""])[i]
            out = examples.get("response", [""])[i]
            text = f"### Instruction:\n{inst}\n{ctx}\n\n### Response:\n{out}"
        elif format_type == "oasst":
            text = examples.get("text", [""])[i]
        elif format_type == "sharegpt":
            conv = examples.get("messages", [[]])[i]
            text = tokenizer.apply_chat_template(conv, tokenize=False, add_generation_prompt=False)
        else:
            text = str(examples)
        texts.append(text)
    return {"text": texts}


def training_thread(model_id, dataset_id, quant, lora_r, lora_alpha, lora_dropout,
                    target_modules, max_seq, batch_size, grad_accum, lr, epochs,
                    scheduler, warmup, output_dir):
    """Background training worker."""
    try:
        _training_state["logs"].append(f"[INIT] Loading model: {model_id}")
        bnb_config = build_bnb_config(quant)
        dtype = torch.bfloat16 if quant in ("FP16", "BF16") else torch.float16

        tokenizer = AutoTokenizer.from_pretrained(model_id, trust_remote_code=True)
        if tokenizer.pad_token is None:
            tokenizer.pad_token = tokenizer.eos_token

        model = AutoModelForCausalLM.from_pretrained(
            model_id,
            quantization_config=bnb_config,
            torch_dtype=dtype,
            device_map="auto",
            trust_remote_code=True,
        )

        arch = MODEL_REGISTRY.get(model_id, {}).get("arch", "llama")
        targets = target_modules if target_modules else DEFAULT_LORA_TARGETS.get(arch, ["q_proj", "v_proj"])

        lora_cfg = LoraConfig(
            r=lora_r,
            lora_alpha=lora_alpha,
            target_modules=targets,
            lora_dropout=lora_dropout,
            bias="none",
            task_type="CAUSAL_LM",
        )
        model = get_peft_model(model, lora_cfg)
        model.print_trainable_parameters()

        _training_state["logs"].append(f"[DATA] Loading dataset: {dataset_id}")
        ds = load_dataset(dataset_id, split="train", trust_remote_code=True)
        fmt = DATASET_REGISTRY.get(dataset_id, {}).get("format", "alpaca")
        ds = ds.map(lambda x: format_dataset(x, fmt, tokenizer), batched=True, remove_columns=ds.column_names)

        args = TrainingArguments(
            output_dir=output_dir,
            num_train_epochs=epochs,
            per_device_train_batch_size=batch_size,
            gradient_accumulation_steps=grad_accum,
            learning_rate=lr,
            warmup_steps=warmup,
            lr_scheduler_type=scheduler,
            logging_steps=5,
            save_strategy="epoch",
            fp16=(quant == "FP16"),
            bf16=(quant == "BF16"),
            optim="paged_adamw_8bit" if quant == "4-bit" else "adamw_torch",
            report_to="none",
            remove_unused_columns=False,
        )

        trainer = SFTTrainer(
            model=model,
            tokenizer=tokenizer,
            train_dataset=ds,
            max_seq_length=max_seq,
            args=args,
            dataset_text_field="text",
        )

        _training_state["trainer"] = trainer
        _training_state["model"] = model
        _training_state["tokenizer"] = tokenizer
        _training_state["logs"].append("[TRAIN] Starting training...")

        trainer.train()
        _training_state["adapter_path"] = output_dir
        _training_state["logs"].append(f"[DONE] Adapter saved to {output_dir}")

    except Exception as e:
        _training_state["logs"].append(f"[ERROR] {str(e)}")


# ── Gradio Interface ─────────────────────────────────────────────────
with gr.Blocks(title="LoRA Fine-Tuning Studio", css="""
    .tabbed { padding: 12px; }
    .metric-box { background: #f5f5f5; border-radius: 8px; padding: 10px; text-align: center; }
    .metric-val { font-size: 22px; font-weight: 600; }
    .metric-lbl { font-size: 12px; color: #666; }
""") as demo:

    gr.Markdown("# 🧬 LoRA Fine-Tuning Studio")
    gr.Markdown("Fine-tune LLMs with LoRA via Hugging Face — model selection, quantization, training, and chat.")

    with gr.Tabs():
        # ── Tab 1: Model & Dataset ─────────────────────────────────
        with gr.TabItem("1. Model & Dataset"):
            with gr.Row():
                with gr.Column(scale=1):
                    model_dropdown = gr.Dropdown(
                        choices=list(MODEL_REGISTRY.keys()),
                        value="meta-llama/Meta-Llama-3-8B-Instruct",
                        label="Base Model",
                    )
                    model_info = gr.JSON(label="Model Info", value=MODEL_REGISTRY["meta-llama/Meta-Llama-3-8B-Instruct"])
                with gr.Column(scale=1):
                    dataset_dropdown = gr.Dropdown(
                        choices=list(DATASET_REGISTRY.keys()),
                        value="tatsu-lab/alpaca",
                        label="Dataset",
                    )
                    dataset_info = gr.JSON(label="Dataset Info", value=DATASET_REGISTRY["tatsu-lab/alpaca"])

            with gr.Row():
                quant_radio = gr.Radio(
                    choices=["4-bit", "8-bit", "FP16", "BF16"],
                    value="4-bit",
                    label="Quantization",
                )
                max_seq_slider = gr.Slider(128, 4096, value=2048, step=128, label="Max Sequence Length")

            def update_info(model, dataset):
                return MODEL_REGISTRY.get(model, {}), DATASET_REGISTRY.get(dataset, {})

            model_dropdown.change(update_info, [model_dropdown, dataset_dropdown], [model_info, dataset_info])
            dataset_dropdown.change(update_info, [model_dropdown, dataset_dropdown], [model_info, dataset_info])

        # ── Tab 2: LoRA Config ─────────────────────────────────────
        with gr.TabItem("2. LoRA Config"):
            with gr.Row():
                lora_r = gr.Slider(1, 256, value=64, step=8, label="LoRA Rank (r)")
                lora_alpha = gr.Slider(1, 512, value=128, step=8, label="LoRA Alpha")
                lora_dropout = gr.Slider(0.0, 0.5, value=0.05, step=0.01, label="LoRA Dropout")

            with gr.Row():
                target_modules = gr.Textbox(
                    value="q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj",
                    label="Target Modules (comma-separated)",
                )
                use_defaults = gr.Checkbox(value=True, label="Use architecture defaults")

            def set_targets(model, use_def):
                if use_def:
                    arch = MODEL_REGISTRY.get(model, {}).get("arch", "llama")
                    return ",".join(DEFAULT_LORA_TARGETS.get(arch, []))
                return target_modules.value

            use_defaults.change(set_targets, [model_dropdown, use_defaults], [target_modules])
            model_dropdown.change(set_targets, [model_dropdown, use_defaults], [target_modules])

            # VRAM Estimator
            vram_json = gr.JSON(label="VRAM Estimate")
            def calc_vram(m, q, r, seq, batch):
                return estimate_vram(m, q, r, seq, batch)
            for comp in [model_dropdown, quant_radio, lora_r, max_seq_slider]:
                comp.change(calc_vram, [model_dropdown, quant_radio, lora_r, max_seq_slider, gr.State(1)], [vram_json])

        # ── Tab 3: Training ────────────────────────────────────────
        with gr.TabItem("3. Training"):
            with gr.Row():
                batch_size = gr.Slider(1, 16, value=1, step=1, label="Per-Device Batch Size")
                grad_accum = gr.Slider(1, 32, value=4, step=1, label="Gradient Accumulation Steps")
                lr = gr.Number(value=2e-4, label="Learning Rate")
            with gr.Row():
                epochs = gr.Slider(1, 10, value=1, step=1, label="Epochs")
                scheduler = gr.Dropdown(
                    choices=["linear", "cosine", "cosine_with_restarts", "polynomial", "constant"],
                    value="cosine",
                    label="LR Scheduler",
                )
                warmup = gr.Slider(0, 500, value=100, step=10, label="Warmup Steps")
            output_dir = gr.Textbox(value="./lora-output", label="Output Directory")

            with gr.Row():
                start_btn = gr.Button("▶ Start Training", variant="primary")
                stop_btn = gr.Button("⏹ Stop", variant="stop")

            log_box = gr.Textbox(label="Training Logs", lines=12, interactive=False)

            # Metrics
            with gr.Row():
                loss_plot = gr.LinePlot(
                    x="step", y="loss", title="Training Loss",
                    height=250, width=400,
                )
                lr_plot = gr.LinePlot(
                    x="step", y="lr", title="Learning Rate",
                    height=250, width=400,
                )

        # ── Tab 4: Chat ────────────────────────────────────────────
        with gr.TabItem("4. Chat"):
            with gr.Row():
                with gr.Column(scale=3):
                    chatbot = gr.Chatbot(label="Fine-Tuned Model", height=400)
                    msg = gr.Textbox(label="Message", placeholder="Type a message...")
                    with gr.Row():
                        send_btn = gr.Button("Send")
                        clear_btn = gr.Button("Clear")
                with gr.Column(scale=1):
                    gr.Markdown("### Load Adapter")
                    adapter_path = gr.Textbox(value="./lora-output", label="Adapter Path")
                    load_adapter_btn = gr.Button("Load Adapter")
                    adapter_status = gr.Textbox(label="Status", interactive=False)

        # ── Tab 5: Export ──────────────────────────────────────────
        with gr.TabItem("5. Export Script"):
            export_box = gr.Code(label="Generated Training Script", language="python", lines=25)

    # ── Event Wiring ───────────────────────────────────────────────────

    def start_train(model, dataset, quant, r, alpha, dropout, targets, seq, batch, accum, lr_val, ep, sched, warm, out):
        _training_state["logs"].clear()
        _training_state["cancelled"] = False
        tlist = [t.strip() for t in targets.split(",") if t.strip()]
        thread = threading.Thread(
            target=training_thread,
            args=(model, dataset, quant, r, alpha, dropout, tlist, seq, batch, accum, lr_val, ep, sched, warm, out),
        )
        thread.start()
        return "Training started in background..."

    start_btn.click(
        start_train,
        [model_dropdown, dataset_dropdown, quant_radio, lora_r, lora_alpha, lora_dropout,
         target_modules, max_seq_slider, batch_size, grad_accum, lr, epochs, scheduler, warmup, output_dir],
        [log_box],
    )

    def poll_logs():
        return "\n".join(_training_state["logs"][-50:])

    demo.load(poll_logs, None, log_box, every=2)

    def stop_train():
        _training_state["cancelled"] = True
        if _training_state["trainer"]:
            _training_state["trainer"].control.should_training_stop = True
        return "Stop signal sent."

    stop_btn.click(stop_train, None, log_box)

    def generate_script(model, dataset, quant, r, alpha, dropout, targets, seq, batch, accum, lr_val, ep, sched, warm, out):
        tlist = [f'"{t.strip()}"' for t in targets.split(",") if t.strip()]
        script = f'''from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig, TrainingArguments
from peft import LoraConfig, get_peft_model
from trl import SFTTrainer
from datasets import load_dataset
import torch

# Config
MODEL_ID = "{model}"
DATASET_ID = "{dataset}"
OUTPUT_DIR = "{out}"

# Quantization
bnb_config = BitsAndBytesConfig(
    load_in_4bit={str(quant == "4-bit").lower()},
    bnb_4bit_compute_dtype=torch.bfloat16,
    bnb_4bit_use_double_quant=True,
    bnb_4bit_quant_type="nf4",
) if "{quant}" == "4-bit" else None

# Load
tokenizer = AutoTokenizer.from_pretrained(MODEL_ID, trust_remote_code=True)
tokenizer.pad_token = tokenizer.eos_token
model = AutoModelForCausalLM.from_pretrained(
    MODEL_ID,
    quantization_config=bnb_config,
    torch_dtype=torch.bfloat16,
    device_map="auto",
    trust_remote_code=True,
)

# LoRA
lora_config = LoraConfig(
    r={r},
    lora_alpha={alpha},
    target_modules=[{", ".join(tlist)}],
    lora_dropout={dropout},
    bias="none",
    task_type="CAUSAL_LM",
)
model = get_peft_model(model, lora_config)

# Data
ds = load_dataset(DATASET_ID, split="train")
# TODO: format ds to {{ "text": ... }} field

# Train
args = TrainingArguments(
    output_dir=OUTPUT_DIR,
    num_train_epochs={ep},
    per_device_train_batch_size={batch},
    gradient_accumulation_steps={accum},
    learning_rate={lr_val},
    warmup_steps={warm},
    lr_scheduler_type="{sched}",
    logging_steps=10,
    save_strategy="epoch",
    fp16={str(quant == "FP16").lower()},
    bf16={str(quant == "BF16").lower()},
    optim="paged_adamw_8bit" if "{quant}" == "4-bit" else "adamw_torch",
)
trainer = SFTTrainer(model=model, tokenizer=tokenizer, train_dataset=ds,
                     max_seq_length={seq}, args=args, dataset_text_field="text")
trainer.train()
model.save_pretrained(OUTPUT_DIR)
'''
        return script

    # Export script generation
    for comp in [model_dropdown, dataset_dropdown, quant_radio, lora_r, lora_alpha, lora_dropout,
                 target_modules, max_seq_slider, batch_size, grad_accum, lr, epochs, scheduler, warmup, output_dir]:
        comp.change(generate_script,
                    [model_dropdown, dataset_dropdown, quant_radio, lora_r, lora_alpha, lora_dropout,
                     target_modules, max_seq_slider, batch_size, grad_accum, lr, epochs, scheduler, warmup, output_dir],
                    [export_box])

    # Chat
    def load_adapter(path):
        if _training_state["model"] is None:
            return "No base model loaded. Train or load one first."
        try:
            _training_state["model"] = PeftModel.from_pretrained(_training_state["model"], path)
            return f"Adapter loaded from {path}"
        except Exception as e:
            return f"Error: {str(e)}"

    load_adapter_btn.click(load_adapter, [adapter_path], [adapter_status])

    def chat(message, history):
        if _training_state["model"] is None or _training_state["tokenizer"] is None:
            return history + [{"role": "assistant", "content": "No model loaded. Please train or load an adapter first."}]
        tok = _training_state["tokenizer"]
        model = _training_state["model"]
        inputs = tok(message, return_tensors="pt").to(model.device)
        with torch.no_grad():
            out = model.generate(**inputs, max_new_tokens=256, do_sample=True, temperature=0.7, top_p=0.9)
        text = tok.decode(out[0], skip_special_tokens=True)
        # Remove prompt echo
        reply = text[len(message):].strip()
        history = history + [{"role": "user", "content": message}, {"role": "assistant", "content": reply}]
        return history

    send_btn.click(chat, [msg, chatbot], [chatbot])
    clear_btn.click(lambda: [], None, chatbot)

    # Init export
    demo.load(generate_script,
              [model_dropdown, dataset_dropdown, quant_radio, lora_r, lora_alpha, lora_dropout,
               target_modules, max_seq_slider, batch_size, grad_accum, lr, epochs, scheduler, warmup, output_dir],
              [export_box])

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860, share=False)