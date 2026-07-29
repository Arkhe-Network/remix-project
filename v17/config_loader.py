import yaml
import os
from pathlib import Path

class CathedralConfig:
    def __init__(self, config_path=None):
        if config_path is None:
            config_path = os.environ.get(
                "CATHEDRAL_CONFIG",
                "config/config.yaml"
            )
        self.config_path = Path(config_path)
        self._config = None
        self.load()

    def load(self):
        if not self.config_path.exists():
            # mock for testing
            self._config = {
                "cathedral": {
                    "version": "17.1",
                    "fast_brain": {
                        "action_dim": 4,
                        "device": "cpu",
                        "safety": {"max_force": 10.0},
                        "memory": {"data_dir": "zvec_data"},
                        "world_model": {"deter_dim": 256, "stoch_dim": 32, "hidden_dim": 256}
                    },
                    "slow_brain": {
                        "swi_reasoning": {"system_prompt": "You are Cathedral."}
                    }
                }
            }
        else:
            with open(self.config_path, "r", encoding="utf-8") as f:
                self._config = yaml.safe_load(f)

    def get(self, path, default=None):
        keys = path.split(".")
        value = self._config.get("cathedral", self._config)
        for key in keys:
            if isinstance(value, dict):
                value = value.get(key, default)
            else:
                return default
        return value

    @property
    def fast_brain(self):
        return self._config["cathedral"]["fast_brain"]

    @property
    def slow_brain(self):
        return self._config["cathedral"]["slow_brain"]
