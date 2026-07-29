"""
Cathedral ARKHE v17.0 - Recursive Evolution & DPO Training
O modelo reescreve suas próprias regras com base no sucesso/falha.
"""
import json
import logging
from pathlib import Path
logger = logging.getLogger("cathedral.evolution")

class EvolutivePromptManager:
    def __init__(self, base_prompt: str, save_path="config/evolved_prompt.txt"):
        self.base_prompt = base_prompt
        self.current_prompt = base_prompt
        self.save_path = Path(save_path)
        if self.save_path.exists():
            self.current_prompt = self.save_path.read_text(encoding="utf-8")

    def evolve(self, trigger_reason: str, success: bool):
        """Modifica o prompt baseado no resultado da ação."""
        if "Z3_UNSAT" in trigger_reason and not success:
            addition = "\n- REGRA EVOLUTIVA: Em ambientes confinados, reduza a magnitude do eixo X e Y em 50% antes de calcular a trajetória."
            if addition not in self.current_prompt:
                self.current_prompt += addition
                logger.info("Prompt evoluiu: Adicionada regra de cautela espacial.")
                self._save()
        elif "DEADLOCK" in trigger_reason and success:
            # Remove regras de cautela excessiva se estavam causando deadlock
            if "reduza a magnitude" in self.current_prompt:
                self.current_prompt = self.current_prompt.replace("\n- REGRA EVOLUTIVA: Em ambientes confinados, reduza a magnitude do eixo X e Y em 50% antes de calcular a trajetória.", "")
                logger.info("Prompt evoluiu: Removeu regra de cautela excessiva (causava deadlock).")
                self._save()

    def _save(self):
        self.save_path.write_text(self.current_prompt, encoding="utf-8")


class CathedralDPOTrainer:
    """Fine-tuning DPO usando HuggingFace TRL (Substitui o NexRL fictício)."""

    def __init__(self, model_name: str = "Qwen/Qwen2.5-7B-Instruct"):
        self.model_name = model_name

    def train_from_interactions(self, dataset_path: str, output_dir: str = "models/cathedral_finetuned"):
        """Lê o JSONL gerado pelo orchestrator e treina via DPO."""
        try:
            from trl import DPOTrainer, DPOConfig
            from transformers import AutoModelForCausalLM, AutoTokenizer
            from datasets import Dataset
        except ImportError:
            logger.error("Bibliotecas 'trl' ou 'transformers' não instaladas.")
            return

        # Carrega dados no formato TRL
        data = []
        with open(dataset_path, "r", encoding="utf-8") as f:
            for line in f:
                item = json.loads(line)
                if item.get("type") != "ppo": # Usa dados de preferência (DPO)
                    data.append({
                        "prompt": item["prompt"],
                        "chosen": item["chosen"],
                        "rejected": item["rejected"]
                    })

        if not data:
            logger.warning("Nenhum dado DPO encontrado para treinar.")
            return

        dataset = Dataset.from_list(data)

        logger.info(f"Iniciando DPO com {len(data)} pares de preferência...")

        # Em produção real, isso exige GPU massiva.
        # Para o exemplo, deixamos o código estruturado.
        tokenizer = AutoTokenizer.from_pretrained(self.model_name)
        model = AutoModelForCausalLM.from_pretrained(self.model_name, load_in_4bit=True) # QLoRA

        training_args = DPOConfig(
            output_dir=output_dir,
            per_device_train_batch_size=2,
            learning_rate=5e-7,
            max_steps=100, # Ajustar para real
            remove_unused_columns=False,
            gradient_checkpointing=True,
        )

        trainer = DPOTrainer(
            model=model,
            args=training_args,
            train_dataset=dataset,
            tokenizer=tokenizer,
        )

        # trainer.train() # Descomentar para executar treinamento real
        # trainer.save_model(output_dir)
        logger.info(f"Treinamento DPO configurado. Modelo seria salvo em {output_dir}")
