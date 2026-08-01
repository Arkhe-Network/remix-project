# Review Process

This document defines the acceptance criteria and code review guidelines for all contributions.

## Acceptance Criteria
- Code must compile without errors or warnings.
- All tests (Rust, Lean, Python) must pass.
- Formal verifications (if applicable) must hold.

## Code Review Guidelines
- **Checklists:** Ensure memory safety, avoid deadlocks (e.g., dropping `RwLock` correctly).
- **Métricas de Qualidade:** Adherence to coding conventions defined in agent rules and project standards.
- **Approvals:** Pull requests require at least one verified approval and CI validation before merge.
