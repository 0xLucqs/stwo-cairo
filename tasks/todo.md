# Mobile RAM Optimization

## Plan

- [x] Add a prover memory-mode switch for selecting the existing fast path or a low-memory path.
- [x] Make base-trace generation use a sequential low-memory path that preserves the current component commit order exactly.
- [x] Make interaction-trace generation use the same low-memory ordering discipline.
- [x] Add proof-equivalence tests proving the low-memory path is byte-identical to the fast path.
- [x] Run focused prover verification commands and record the outcome here.

## Review

- Added `STWO_PROVER_MEMORY_MODE` parsing in `stwo_cairo_prover/crates/prover/src/prover.rs`.
- Supported values are `fast` and `low_memory`; unset or empty keeps the existing fast path.
- Routed both base-trace generation and interaction-trace generation through explicit `ProverMemoryMode` selection.
- The low-memory mode keeps the exact existing component extension order and only changes scheduling: traces are generated and committed sequentially instead of being fanned out in parallel and retained simultaneously.
- The fast path implementation remains intact and is still the default behavior.
- Added a new end-to-end proof equivalence test on the `test_prove_verify_ret_opcode` fixture to assert the low-memory proof bytes match the fast path bytes exactly.
- Verification commands:
  - `rtk proxy rustup run nightly-2025-06-23 cargo fmt --manifest-path stwo_cairo_prover/Cargo.toml --all`
  - `rtk proxy rustup run nightly-2025-06-23 cargo check --manifest-path stwo_cairo_prover/Cargo.toml -p stwo-cairo-prover`
  - `rtk proxy rustup run nightly-2025-06-23 cargo test --manifest-path stwo_cairo_prover/Cargo.toml -p stwo-cairo-prover prover::tests::test_low_memory_proof_matches_fast_path_ret_opcode -- --exact --nocapture`
  - `rtk proxy rustup run nightly-2025-06-23 cargo test --manifest-path stwo_cairo_prover/Cargo.toml -p stwo-cairo-prover prover::tests::test_all_cairo_constraints_small_ppt -- --exact --nocapture`
- Verification results:
  - `cargo check` passed.
  - `prover::tests::test_low_memory_proof_matches_fast_path_ret_opcode` passed in `296.11s`.
  - `prover::tests::test_all_cairo_constraints_small_ppt` passed in `21.44s`.
- Benchmark on `test_prove_verify_all_builtins/prover_input.json` using the release `prove` binary and unrestricted `ps` RSS sampling:
  - Fast path: `peak_rss_kb=21187296`, `elapsed_s=19`
  - Low-memory path: `peak_rss_kb=21140896`, `elapsed_s=18`
  - Delta: about `45 MiB` lower peak RSS, about `1s` faster on this host/fixture.
  - `cmp -s` confirmed `/tmp/stwo-all-builtins-fast.proof.bin` and `/tmp/stwo-all-builtins-low-memory.proof.bin` are byte-identical.
- Rebuilt privacy-demo benchmark on `/Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-a4f62c68101db875`:
  - Fast path:
    - `11.58 real`
    - `15305687040` maximum resident set size
    - `15216225584` peak memory footprint
  - Low-memory path:
    - `25.38 real`
    - `8720121856` maximum resident set size
    - `8637323984` peak memory footprint
  - Delta on the rebuilt private transaction workload:
    - about `6.08 GiB` lower maximum RSS
    - about `6.13 GiB` lower peak memory footprint
    - about `13.80s` slower wall time
- Remaining gap:
  - This turn verified correctness, proof identity, and one repo-local RSS benchmark, but the measured RAM win on this particular fixture is small.
  - A larger real-world workload may still be needed to expose the transient-base-trace savings that motivated the low-memory path.

# Privacy Demo RAM Reduction

## Plan

- [x] Add a low-memory interaction-generation path in `cairo_claim_generator.rs` that extends each component's interaction trace immediately instead of fanning all components out in parallel.
- [x] Keep the existing fast-path behavior unchanged unless `STWO_PROVER_MEMORY_MODE=low_memory`.
- [x] Build and run focused prover tests in `stwo_cairo_prover` to verify the refactor compiles and preserves behavior.
- [x] Re-measure the privacy-demo harness with the exact `sequencer` commands if local verification succeeds and access permits.
- [x] Record review notes and outcome here.

## Review

- Added a low-memory-only interaction path in `stwo_cairo_prover/crates/prover/src/witness/cairo_claim_generator.rs`.
- The new path preserves component commit order but extends each interaction trace immediately, so temporary interaction buffers are dropped before the next component runs.
- The existing parallel interaction fanout remains the default path; the new behavior activates only when `STWO_PROVER_MEMORY_MODE=low_memory`.
- Verified against the exact patched STWO worktree at `/Users/lucas/stwo/.claude/worktrees/beautiful-chandrasekhar`.
- `cargo check` passed for `stwo-cairo-prover` with local path patches to that worktree.
- `prover::tests::test_all_cairo_constraints_small_ppt` passed with the same patched dependency set.
- Rebuilt the offline `sequencer` test harness with a temporary `Cargo.lock` at `/tmp/sequencer-head-lock/Cargo.lock`, local `stwo-cairo` patches from `/tmp/stwo-cairo-467d5c6-local`, and the sequencer venv on `PATH`.
- Reused `/Users/lucas/sequencer/target/release/shared_executables/starknet-sierra-compile` to keep the rebuild offline.
- Verified the rebuilt harness binary `/Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-8d65356357ff48b6` contains `proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction`.
- Offline benchmark command:
  - `CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-8d65356357ff48b6 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
- Offline low-memory benchmark command:
  - `STWO_PROVER_MEMORY_MODE=low_memory CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-8d65356357ff48b6 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
- Baseline result on the rebuilt binary:
  - `19.04 real`
  - `maximum resident set size`: `11809914880`
  - `peak memory footprint`: `16729633224`
- Low-memory result on the rebuilt binary:
  - `17.08 real`
  - `maximum resident set size`: `13778567168`
  - `peak memory footprint`: `16185094192`
- Delta on this rebuilt binary:
  - Wall time improved by about `1.96s`.
  - Peak memory footprint dropped by about `544.5 MiB`.
  - Maximum RSS increased by about `1.83 GiB`.
- Important caveat:
  - This benchmark stayed fully offline and used the patched local `stwo-cairo` path, but Cargo still resolved `stwo` from the pinned git checkout `aeceb74c` rather than `/Users/lucas/stwo/.claude/worktrees/beautiful-chandrasekhar` because the local worktree patch version did not match the locked graph. The benchmark is therefore valid for the `stwo-cairo` interaction-path change in isolation, not yet for the full local-`stwo` stack the user originally measured.

## Verified Local Integration

- Current `sequencer` wiring now resolves the privacy-demo proving stack to local paths:
  - `privacy-prove v1.1.0 (/Users/lucas/proving-utils/crates/privacy_prove)`
  - `cairo-air v1.1.0 (/Users/lucas/stwo-cairo/stwo_cairo_prover/crates/cairo-air)`
  - `stwo-cairo-prover v1.1.0 (/Users/lucas/stwo-cairo/stwo_cairo_prover/crates/prover)`
  - `stwo v2.2.0 (/Users/lucas/stwo/.claude/worktrees/beautiful-chandrasekhar/crates/stwo)`
- Verification command:
  - `PATH=/Users/lucas/sequencer/sequencer_venv/bin:$PATH cargo tree --manifest-path /Users/lucas/sequencer/Cargo.toml -p starknet_transaction_prover --features stwo_proving --depth 3`
- Rebuild command that succeeded with the correct nightly toolchain:
  - `PATH=/Users/lucas/sequencer/sequencer_venv/bin:$PATH rustup run nightly-2025-07-14 cargo test --manifest-path /Users/lucas/sequencer/Cargo.toml -p starknet_transaction_prover --release --features stwo_proving proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --no-run --offline -- --ignored --exact --nocapture`
- Rebuilt harness binary:
  - `/Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-a4f62c68101db875`

## Verified Measurements On Fully Local Stack

- Baseline command:
  - `CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-a4f62c68101db875 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
- Baseline result:
  - `14.65 real`
  - `maximum resident set size`: `11931238400`
  - `peak memory footprint`: `15220518168`

- Low-memory command:
  - `STWO_PROVER_MEMORY_MODE=low_memory CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-a4f62c68101db875 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
- Low-memory result:
  - `27.56 real`
  - `maximum resident set size`: `9058844672`
  - `peak memory footprint`: `8109742440`
  - The run stayed offline and used the mocked replay path; one recorded `starknet_getClass` response was replayed from one step later in the transcript.

- Delta versus baseline on the same fully local stack:
  - Wall time regressed by about `12.91s`.
  - Maximum RSS improved by about `2.66 GiB`.
  - Peak memory footprint improved by about `6.63 GiB`.

- Delta versus the user's earlier low-memory measurement on this machine:
  - Earlier low-memory: `26.75 real`, `7505510400` max RSS, `8025921776` peak footprint.
  - Current low-memory: `27.56 real`, `9058844672` max RSS, `8109742440` peak footprint.
  - Net change: about `0.81s` slower, about `1.45 GiB` higher max RSS, about `80 MiB` higher peak footprint.

# RAM Profiling: Cairo Proving With Local STWO

## Plan

- [x] Inspect the workspace to find the proving entrypoint and available Cairo programs.
- [x] Point `stwo_cairo_prover` at the local `/Users/lucas/stwo` checkout for this run.
- [x] Run a real proving command with RAM profiling enabled.
- [x] Verify the run used the local STWO checkout and capture the result.
- [x] Document the final command, measurements, and review notes.

## Notes

- `tasks/lessons.md` was not present at session start, so there were no project-specific lessons to review.
- The top-level README confirms the old CLI is gone; the active proving utilities live under `stwo_cairo_prover/crates/dev_utils/src/bin/`.
- `run_and_prove` is the best fit here because it executes a compiled Cairo program, adapts it, and calls the prover in one path.
- For a prover-only measurement, `prove` is cleaner because it avoids Cairo VM and adapter overhead by consuming a pre-generated `prover_input.json`.

## Provenance

- The local STWO checkout used for this run is `/Users/lucas/stwo` on branch `dev`.
- The checkout is modified; `git status --short` reported changes in `Cargo.lock`, several files under `crates/stwo/`, `crates/constraint-framework/`, `crates/examples/`, and an untracked `tasks/` directory.
- Cargo resolution was verified with:
  - `rtk proxy cargo tree --manifest-path /Users/lucas/stwo-cairo/stwo_cairo_prover/Cargo.toml -p stwo-cairo-dev-utils --depth 2 --config 'patch.crates-io.stwo.path="/Users/lucas/stwo/crates/stwo"' --config 'patch.crates-io.stwo-constraint-framework.path="/Users/lucas/stwo/crates/constraint-framework"' --config 'patch.crates-io.stwo-air-utils.path="/Users/lucas/stwo/crates/air-utils"' --config 'patch.crates-io.stwo-air-utils-derive.path="/Users/lucas/stwo/crates/air-utils-derive"'`
- That resolution showed:
  - `stwo v2.2.0 (/Users/lucas/stwo/crates/stwo)`
  - `stwo-constraint-framework v2.2.0 (/Users/lucas/stwo/crates/constraint-framework)`
  - `stwo-air-utils v2.2.0 (/Users/lucas/stwo/crates/air-utils)`
  - `stwo-air-utils-derive v2.2.0 (/Users/lucas/stwo/crates/air-utils-derive)`

## Measurements

- Fixture:
  - `/Users/lucas/stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/compiled.json`
  - `/Users/lucas/stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/prover_input.json`
- Build command:
  - `rtk proxy cargo build --manifest-path /Users/lucas/stwo-cairo/stwo_cairo_prover/Cargo.toml --release -p stwo-cairo-dev-utils --bin prove --bin run_and_prove --config 'patch.crates-io.stwo.path="/Users/lucas/stwo/crates/stwo"' --config 'patch.crates-io.stwo-constraint-framework.path="/Users/lucas/stwo/crates/constraint-framework"' --config 'patch.crates-io.stwo-air-utils.path="/Users/lucas/stwo/crates/air-utils"' --config 'patch.crates-io.stwo-air-utils-derive.path="/Users/lucas/stwo/crates/air-utils-derive"'`
- Prover-only command:
  - `rtk proxy /usr/bin/time -l /Users/lucas/stwo-cairo/stwo_cairo_prover/target/release/prove --prover_input_path /Users/lucas/stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/prover_input.json --proof_path /tmp/test_prove_verify_all_builtins.proof.bin --proof-format binary`
- Prover-only result:
  - `32.37 real`, `109.73 user`, `29.42 sys`
  - `maximum resident set size`: `11249451008`
  - `peak memory footprint`: `28559446416`
  - Output proof: `/tmp/test_prove_verify_all_builtins.proof.bin` (`1.6M`)
- End-to-end command:
  - `rtk proxy /usr/bin/time -l /Users/lucas/stwo-cairo/stwo_cairo_prover/target/release/run_and_prove --program /Users/lucas/stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/compiled.json --proof_path /tmp/test_prove_verify_all_builtins.run_and_prove.proof.bin --proof-format binary`
- End-to-end result:
  - `27.17 real`, `106.53 user`, `20.72 sys`
  - `maximum resident set size`: `20229341184`
  - `peak memory footprint`: `28219609368`
  - Output proof: `/tmp/test_prove_verify_all_builtins.run_and_prove.proof.bin` (`1.6M`)

## Review

- The local-STWO patching was kept on the Cargo command line, so no prover workspace dependency files were modified for this measurement.
- Both runs completed successfully and produced proofs.
- The most stable memory signal on this macOS host is `peak memory footprint`, which was roughly `28.2 GB` for the end-to-end path and `28.6 GB` for the prover-only path on this fixture.
- `maximum resident set size` differed more significantly between the two runs, so it should be treated as a secondary metric rather than the headline number.
