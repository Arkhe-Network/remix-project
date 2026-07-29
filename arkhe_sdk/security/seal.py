import hashlib
import json

class Seal:
    def compute(self, data: dict) -> str:
        json_str = json.dumps(data, sort_keys=True, default=str)
        return hashlib.sha3_256(json_str.encode()).hexdigest()
