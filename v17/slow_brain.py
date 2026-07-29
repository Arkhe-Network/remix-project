import logging
logger = logging.getLogger("cathedral.slow_brain")

class SlowBrainSGLang:
    def __init__(self, config):
        self.config = config
        self.sys_prompt = config.get("slow_brain.swi_reasoning.system_prompt")

    async def health_check(self):
        return False

    async def reason(self, dilemma, context=""):
        return {"action_vector": [0.0, 0.0, 0.0, 0.0], "confidence": 0.0, "reasoning": "Fallback reason"}
