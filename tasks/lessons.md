# Lessons

- When the user names a specific local dependency worktree, use that exact path for all patching and verification commands; do not substitute a nearby checkout.
- "Offline" for this project means no live RPC dependence in the workload path. Prefer saved JSON fixtures and mocked RPC responses over generic cargo offline checks.
