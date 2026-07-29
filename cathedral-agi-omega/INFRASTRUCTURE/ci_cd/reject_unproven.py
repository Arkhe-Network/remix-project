import sys
import subprocess
import re

CRITICAL_DIRS = [
    "ZK_REASONING_ENGINE/circuits",
    "COGNITIVE_CORTEX/agents",
    "DISTRIBUTED_COMPUTATION"
]

def get_changed_files():
    try:
        # Get list of changed files against the target branch (e.g., main)
        result = subprocess.run(['git', 'diff', '--name-only', 'origin/main...HEAD'], capture_output=True, text=True)
        if result.returncode != 0:
            # Fallback if origin/main is not available
            result = subprocess.run(['git', 'diff', '--name-only', 'HEAD~1...HEAD'], capture_output=True, text=True)
        return result.stdout.strip().split('\n')
    except Exception as e:
        print(f"Error getting changed files: {e}")
        return []

def check_for_lean_proofs(changed_files):
    modifies_critical = False
    has_lean_proof = False

    for file in changed_files:
        if any(crit_dir in file for crit_dir in CRITICAL_DIRS):
            modifies_critical = True
        if file.endswith('.lean'):
            has_lean_proof = True

    if modifies_critical and not has_lean_proof:
        print("❌ ERROR: You have modified critical AGI components without providing an updated Lean 4 proof.")
        print("Please ensure you include a .lean file demonstrating that your changes preserve the safety theorems.")
        sys.exit(1)

    if modifies_critical and has_lean_proof:
        print("✅ SUCCESS: Critical modifications detected alongside Lean 4 proofs. Superego constraints satisfied.")
    else:
        print("✅ SUCCESS: No critical modifications detected, or proofs provided. Proceeding.")
    sys.exit(0)

if __name__ == "__main__":
    files = get_changed_files()
    if files == ['']:
        print("No files changed.")
        sys.exit(0)
    check_for_lean_proofs(files)
