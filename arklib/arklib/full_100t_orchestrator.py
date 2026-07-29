"""Substrato 989.y.3 - FULL-100T-ORCHESTRATOR"""

import sys
import os
import importlib.util

# This is a stub module to expose the orchestrator from its own package.
# We don't want to copy the whole file into arklib to maintain single source of truth.

_target_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "arkhe-substrato-989y3-full-100t-orchestrator", "full_100t_orchestrator.py"))

if os.path.exists(_target_path):
    spec = importlib.util.spec_from_file_location("full_100t_orchestrator", _target_path)
    if spec and spec.loader:
        module = importlib.util.module_from_spec(spec)
        sys.modules["full_100t_orchestrator"] = module
        spec.loader.exec_module(module)

        # Expose everything from the module here
        from full_100t_orchestrator import *
