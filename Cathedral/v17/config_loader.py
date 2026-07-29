import yaml
import os
from pathlib import Path

class CathedralConfig:
    def __init__(self, config_path=None):
        if config_path is None:
            config_path = os.environ.get(
                "CATHEDRAL_CONFIG",
                "Cathedral/config/config.yaml"
            )
        self.config_path = Path(config_path)
        self._config = None
        self.load()

    def load(self):
        if not self.config_path.exists():
            raise FileNotFoundError(
                f"Config não encontrado: {self.config_path}"
            )
        with open(self.config_path, "r", encoding="utf-8") as f:
            self._config = yaml.safe_load(f)
        self._validate()

    def _validate(self):
        c = self._config.get("cathedral", self._config)
        assert "fast_brain" in c, "fast_brain ausente"
        assert "slow_brain" in c, "slow_brain ausente"
        assert "router" in c, "router ausente"

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

    @property
    def router(self):
        return self._config["cathedral"]["router"]

    @property
    def zvec(self):
        return self._config["cathedral"]["zvec"]

    @property
    def telemetry(self):
        return self._config["cathedral"]["telemetry"]

    def __repr__(self):
        return f"CathedralConfig(v{self._config['cathedral']['version']}, path={self.config_path})"


if __name__ == "__main__":
    config = CathedralConfig()
    print(config)
    print(f"Fast Brain device: {config.get('fast_brain.device')}")
    print(f"Slow Brain engine: {config.get('slow_brain.engine')}")
    print(f"Slow Brain endpoint: {config.get('slow_brain.api_base')}")
    print(f"Memory backend: {config.get('fast_brain.memory.backend')}")
    print(f"XGrammar: {config.get('slow_brain.xgrammar.enabled')}")
