# Project Memory

This document acts as the persistent context and knowledge base for the project, tracking important decisions, context, and lessons learned.

## Architectural Decision Records (ADRs)
- Store detailed records in `docs/architecture/ADR`.
- Summary of major shifts (e.g., adoption of Zero Token Architecture).

## Lessons Learned
- Avoid deadlocks by explicitly dropping `RwLock` guards before async calls.
- Track precise diagnostics using tools like `pretty_assertions`.
