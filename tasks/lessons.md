# Lessons

- When asked whether a fast path is unchanged, do not infer it from local fast-vs-low-memory equality alone. Either compare against a clean checkout at the same commits or say explicitly that vanilla-vs-local has not been verified yet.
- When the user names a specific local dependency worktree, use that exact path for all patching and verification commands; do not substitute a nearby checkout.
- "Offline" for this project means no live RPC dependence in the workload path. Prefer saved JSON fixtures and mocked RPC responses over generic cargo offline checks.
- For mobile RAM work on this project, optimize and report the iOS-jetsam-style peak memory footprint as the primary metric; max RSS is only secondary context.
- When reporting Instruments allocation data, distinguish end-live bytes from cumulative allocation volume. Do not present `total-bytes` or end-of-run live rows as peak memory.
- When the user asks to profile a specific workload, keep the exact target workload unchanged. Do not substitute a different local harness just because the command shape is similar.
- When optimizing mobile memory for this project, prioritize transient peak owners over long-lived survivors. A trace that cannot attribute the high-water mark is not enough to choose the next optimization.
- When reading Instruments screenshots, use the selected timestamp and visible stack to identify the peak owner. Do not infer the phase from the compressed top graph alone.
- When the user says they rebuilt a target, invalidate any previously profiled copy and rerun measurements on the exact rebuilt binary path they provide.
- Do not hand over an Instruments trace bundle as usable if `xctrace` reports an attach failure or the target stdout file is empty. Re-record using the last known-good launch/signing method and validate the bundle first.
