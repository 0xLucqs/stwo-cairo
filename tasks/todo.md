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

# Fast-Path Regression Fix

## Plan

- [x] Inspect the fast-path panic site and trace it back to the evaluation-retention flow.
- [x] Gate the early evaluation release in `stwo-cairo` so it only runs in low-memory mode.
- [x] Rebuild the sequencer privacy-demo prover target against the patched local dependency.
- [x] Re-run the fresh fast-path and low-memory privacy-demo commands to verify behavior.
- [x] Record the root cause and verification results here.

## Review

- Root cause:
  `stwo_cairo_prover/crates/prover/src/prover.rs` was calling
  `commitment_scheme.release_recomputable_evaluations()` unconditionally right after the base
  trace commit.
- Why that broke only fast mode:
  in core `stwo`, the low-memory proving flow rematerializes evaluations on demand during
  composition generation, but the fast path assumes those evaluation buffers are still retained.
  Releasing them early therefore caused the fast path to panic later in
  `/Users/lucas/stwo/crates/stwo/src/prover/air/component_prover.rs:100` with
  `evaluation buffer is not retained for this polynomial`.
- Fix:
  gated the early release in `stwo_cairo_prover/crates/prover/src/prover.rs` so it runs only when
  `commitment_scheme.memory_mode == ProverMemoryMode::LowMemory`.
- Formatting:
  `rustup run nightly-2025-07-14 rustfmt /Users/lucas/stwo-cairo/stwo_cairo_prover/crates/prover/src/prover.rs`
- Rebuild:
  `PATH=/Users/lucas/sequencer/sequencer_venv/bin:$PATH rustup run nightly-2025-06-20 cargo test --manifest-path /Users/lucas/sequencer/Cargo.toml --release -p starknet_transaction_prover --features stwo_proving --no-run`
- Verified fast path on the exact rebuilt sequencer workload:
  - Command:
    `CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
  - Result:
    - passed
    - `15.25 real`
    - `11433017344` maximum resident set size
    - `15204363544` peak memory footprint
- Re-verified low-memory on the same rebuilt sequencer workload:
  - Command:
    `STWO_PROVER_MEMORY_MODE=low_memory CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
  - Result:
    - passed
    - `56.78 real`
    - `10469801984` maximum resident set size
    - `7484873064` peak memory footprint
- Delta on the fixed build:
  - low-memory is about `41.53s` slower wall time
- low-memory reduces max RSS by about `919 MiB`
- low-memory reduces peak memory footprint by about `7.17 GiB`

# Next RAM Reduction

## Plan

- [x] Inspect the current interaction-generation path to confirm the remaining low-memory fanout.
- [x] Rebuild the sequencer privacy-demo prover target against the patched local dependency.
- [x] Re-run the exact fast-path and low-memory privacy-demo commands and compare peak RAM.
- [x] Attempt the next targeted retention reduction on the heaviest lookup components.
- [x] Record the measured outcome and whether the optimization was safe to keep.

## Review

- Restored the worktree to the verified baseline after the earlier no-gain scheduling experiment in `cairo_claim_generator.rs`; only the fast-path retention fix in `stwo_cairo_prover/crates/prover/src/prover.rs` remains as a code change.
- Freshly rebuilt the exact sequencer target again:
  - `rustup run nightly-2025-06-20 cargo clean --manifest-path /Users/lucas/sequencer/Cargo.toml`
  - `PATH=/Users/lucas/sequencer/sequencer_venv/bin:$PATH rustup run nightly-2025-06-20 cargo test --manifest-path /Users/lucas/sequencer/Cargo.toml --release -p starknet_transaction_prover --features stwo_proving --no-run`
- Reconfirmed the current baseline on `/Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716`:
  - Fast path:
    - `17.31 real`
    - `11534811136` maximum resident set size
    - `15266262368` peak memory footprint
  - Low-memory path:
    - `55.89 real`
    - `10472423424` maximum resident set size
    - `8070027576` peak memory footprint
- Attempted a more structural RAM reduction by compacting retained interaction state in:
  - `stwo_cairo_prover/crates/prover/src/witness/components/range_check_20.rs`
  - `stwo_cairo_prover/crates/prover/src/witness/components/range_check_9_9.rs`
  - `stwo_cairo_prover/crates/prover/src/witness/components/pedersen_points_table_window_bits_18.rs`
- The idea was to stop retaining fully materialized lookup tuples and instead keep only multiplicities plus preprocessed-trace access, then recompute the lookup tuples during interaction generation.
- That refactor compiled, but it was not safe:
  - `cargo check` passed.
  - A direct fast-path proof generation on `test_prove_verify_ret_opcode/compiled.json` succeeded.
  - The matching low-memory run regressed and panicked in `/Users/lucas/stwo/crates/stwo/src/prover/air/component_prover.rs:100` with `evaluation buffer is not retained for this polynomial`.
- Because the regression hit the low-memory proving path itself, the component refactor was fully reverted and not kept.
- Conclusion from this round:
  - there is a plausible RAM win in shrinking retained lookup state for hot components like `range_check_9_9`, `range_check_20`, and `pedersen_points_table_window_bits_18`,
  - but a naive “recompute from preprocessed columns later” rewrite is not safe enough to land as-is,
  - so the repo remains on the last known good baseline while the next optimization will need a deeper understanding of how low-memory rematerialization interacts with these components.

# Instruments Allocation Profile

## Plan

- [x] Capture an Instruments `Allocations` trace for the exact low-memory privacy-demo prover workload.
- [x] Export the allocation tables from the trace bundle and rank the dominant persistent allocation classes.
- [x] Record the profiling result and convert it into concrete mobile-footprint targets.

## Review

- Direct `xctrace` launch/attach against the fresh sequencer test binary failed repeatedly with `Failed to attach to target process`.
- A temporary copy of the exact binary was created at `/tmp/starknet_transaction_prover-profiled` and ad-hoc re-signed with the `com.apple.security.get-task-allow` entitlement. That unblocked a successful `Allocations` recording without modifying the real build artifact.
- Successful trace command:
  - `xcrun xctrace record --template Allocations --output /tmp/privacy-demo-low-memory-profiled.trace --run-name low_memory_privacy_demo_profiled --time-limit 2m --target-stdout /tmp/privacy-demo-low-memory-profiled.stdout --env STWO_PROVER_MEMORY_MODE=low_memory --env CHAIN_ID=SN_INTEGRATION_SEPOLIA --no-prompt --launch -- /tmp/starknet_transaction_prover-profiled proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
- The profiled target run passed:
  - `test proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction ... ok`
  - `finished in 91.12s`
- Exported trace artifacts:
  - `/tmp/privacy-demo-low-memory-profiled.toc.xml`
  - `/tmp/privacy-demo-low-memory-profiled.statistics.xml`
  - `/tmp/privacy-demo-low-memory-profiled.allocations.xml`
- The strongest signal from Instruments is that low-memory mobile footprint is dominated by malloc-backed VM regions, not file-backed mappings or thread stacks:
  - `All VM Regions`: `6780747776` persistent bytes
  - `VM: MALLOC_LARGE`: `3420405760` persistent bytes
  - `VM: MALLOC_SMALL`: `3166699520` persistent bytes
  - `All Heap & Anonymous VM`: `194971376` persistent bytes
  - `All Anonymous VM`: `173195264` persistent bytes
  - `VM: Stack`: `72269824` persistent bytes
  - `VM: Mapped File`: `131072` persistent bytes
- The highest churn classes are also allocator-driven and heavily power-of-two sized:
  - `Malloc 4,00 KiB`: `16367927296` total bytes
  - `Malloc 4,00 MiB`: `8111783936` total bytes
  - `Malloc 16,00 KiB`: `4658331648` total bytes
  - `Malloc 32,00 KiB`: `4010606592` total bytes
  - `Malloc 8,00 KiB`: `3303432192` total bytes
- Source-attributed live allocations from the exported `Allocations List` are much smaller than the total malloc zones, but they still point in the expected direction:
  - `rayon::iter::plumbing::Folder::consume_iter`: `56 x 128 KiB` live allocations (`7340032` bytes total)
  - `alloc::raw_vec::finish_grow`: one `4 MiB` live allocation and one `640 KiB` live allocation
  - `hashbrown::HashMap::insert`: one `1.89 MiB` live allocation, one `1.66 MiB` live allocation, plus smaller retained tables
  - `std::sys::pal::unix::thread::{new,join}` stacks: about `61.7 MiB` live
  - `serde_json` / Cairo JSON parsing: well under `2 MiB` live in aggregate by the end of the run
- Practical conclusion for the next optimization round:
  - the trace does show that allocator-backed VM dominates end-of-run live memory categories,
  - but this CLI export is not sufficient to identify the true high-water-mark owners,
  - because the exported `Allocations List` contains only end-live rows and the `total-bytes` fields are lifetime allocation churn, not peak-resident bytes.
- Corrected interpretation of this capture:
  - `Allocations List` exported `59189` rows and every row had `live="true"`, so it represents end-of-run survivors only.
  - Summing those rows gives only `184324368` bytes live at the end of the run, with `83219280` bytes attributed to `starknet_transaction_prover-profiled`.
  - That is far below the observed low-memory mobile footprint peak of roughly `7.5` to `8.1 GB`, which means the real footprint peak is dominated by transient allocations that were released before process exit.
  - Therefore this specific CLI export can rule out end-of-run retained stacks / JSON parsing / small hash maps as the primary mobile issue, but it cannot yet prove which prover phase owns the actual high-water mark.

# Recursive Precompute Peak Reduction

## Plan

- [x] Patch recursive prover precompute tree construction to honor `STWO_PROVER_MEMORY_MODE`.
- [x] Make borrowed precomputed trees safe to use in low-memory mode by rematerializing evaluations only while each proof is using them.
- [x] Rebuild the sequencer target from scratch and rerun the exact privacy-demo workload.
- [x] Verify the fast path still passes on the same fresh binary.

## Review

- Instruments on the correct sequencer privacy-demo trace showed the early peak was dominated by repeated `Malloc 32 MiB` allocations rooted in:
  - `privacy_prove::prepare_recursive_prover_precomputes`
  - `stwo::prover::pcs::CommitmentTreeProver`
  - `stwo::prover::poly::circle::ops`
- The direct cause in code was that `prepare_recursive_prover_precomputes()` in `/Users/lucas/proving-utils/crates/privacy_prove/src/lib.rs` built both recursive preprocessed trees via `CommitmentTreeProver::new(...)`, which hardwires `ProverMemoryMode::Fast`.
- First attempt:
  - switching those trees directly to `new_with_memory_mode(..., ProverMemoryMode::LowMemory)` reduced the peak but broke the low-memory proof path with `evaluation buffer is not retained for this polynomial`.
  - Root cause: borrowed preprocessed trees were later passed into the PCS as `MaybeOwned::Borrowed`, and the low-memory rematerialization path in `stwo` only rematerialized evaluations for owned trees.
- Final fix:
  - `privacy_prove::prepare_recursive_prover_precomputes()` now parses the current `STWO_PROVER_MEMORY_MODE` and builds both recursive preprocessed trees with `new_with_memory_mode(...)`.
  - The cached preprocessed trees are now stored behind `Mutex<CommitmentTreeProver<...>>` in `RecursiveProverPrecomputes`.
  - `stwo::prover::pcs::CommitmentTreeProver` now exposes `materialize_evaluations_for_reuse(...)` so callers with borrowed low-memory trees can rematerialize evaluations explicitly when needed.
  - `privacy_recursive_prove()` now:
    - locks each cached preprocessed tree,
    - rematerializes its evaluations just before the corresponding proof step,
    - borrows it into the existing proving code unchanged,
    - and drops those evaluations again afterward in low-memory mode.
- Fresh rebuild:
  - `rm -rf /Users/lucas/sequencer/target/release`
  - `PATH=/Users/lucas/sequencer/sequencer_venv/bin:$PATH rustup run nightly-2025-06-20 cargo test --manifest-path /Users/lucas/sequencer/Cargo.toml --release -p starknet_transaction_prover --features stwo_proving --no-run`
- Verified low-memory command on the fresh binary `/Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716`:
  - `STWO_PROVER_MEMORY_MODE=low_memory CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
  - Result:
    - passed
    - `63.36 real`
    - `8881618944` maximum resident set size
    - `6425744640` peak memory footprint
- Verified fast-path command on the same fresh binary:
  - `CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l /Users/lucas/sequencer/target/release/deps/starknet_transaction_prover-90dc1c60897b5716 proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
  - Result:
    - passed
    - `13.41 real`
    - `11744559104` maximum resident set size
    - `15255219504` peak memory footprint
- Measured impact on the mobile-relevant metric:
  - versus the earlier fresh low-memory baseline `8070027576`, peak footprint improved by `1644282936` bytes, about `1.53 GiB` lower.
  - versus the earlier best observed low-memory baseline `7484873064`, peak footprint improved by `1059128424` bytes, about `1.01 GiB` lower.
- Conclusion:
  - the trace-identified recursive precompute trees were a real peak owner,
  - switching them to low-memory safely delivers roughly the expected `~1 to 1.5 GiB` mobile-footprint win on the privacy-demo workload,
  - and the shared fast path remains healthy on the same fresh build.

# Direct Prove Cairo Low-Memory Proof Equality

## Plan

- [x] Restore the `ret_opcode` fast-vs-low-memory proof equality test inside `stwo-cairo`.
- [x] Fix direct `prove_cairo()` preprocessed-tree construction so it respects the active prover memory mode.
- [x] Run the restored proof-equality test and record the outcome.

## Review

- Reintroduced `prover::tests::test_low_memory_proof_matches_fast_path_ret_opcode` in `/Users/lucas/stwo-cairo/stwo_cairo_prover/crates/prover/src/prover.rs` using the same serialized proof-file format as the existing end-to-end tests for `test_prove_verify_ret_opcode`.
- The first run exposed a real low-memory gap in direct `prove_cairo()`: it still built the preprocessed tree with `CommitmentTreeProver::new(...)`, which hardwired the fast path and left low-memory runs without retained coefficients for rematerialization.
- `prove_cairo()` now builds that preprocessed tree with `new_with_memory_mode(...)` based on the active `STWO_PROVER_MEMORY_MODE`, so direct low-memory proving uses the same memory-mode-aware tree construction as the rest of the prover.
- Verification command:
  - `rtk proxy rustup run nightly-2025-06-20 cargo test --manifest-path /Users/lucas/stwo-cairo/stwo_cairo_prover/Cargo.toml -p stwo-cairo-prover --features slow-tests prover::tests::test_low_memory_proof_matches_fast_path_ret_opcode -- --exact --nocapture`
- Verification result:
  - `prover::tests::test_low_memory_proof_matches_fast_path_ret_opcode` passed.
  - Finished in `712.56s`.
  - Outcome: the serialized `ret_opcode` proof is byte-identical between `fast` and `low_memory`.

# Fast Path Vanilla Comparison

## Plan

- [x] Review local diffs for any changes that could affect fast mode.
- [x] Produce a fast-path proof from the current local tree.
- [x] Produce the same fast-path proof from clean temporary checkouts of `stwo-cairo` and `stwo` at the same commits.
- [x] Compare the two proof files byte-for-byte.

## Review

- Fast-path-relevant local changes in `/Users/lucas/stwo-cairo/stwo_cairo_prover/crates/prover/src/prover.rs` are:
  - gating `release_recomputable_evaluations()` so it only runs in low-memory mode,
  - and making direct `prove_cairo()` build its preprocessed tree with `new_with_memory_mode(...)` based on `STWO_PROVER_MEMORY_MODE`.
- The local diff in `/Users/lucas/stwo/crates/stwo/src/prover/pcs/mod.rs` is formatting plus the borrowed-tree helper `materialize_evaluations_for_reuse(...)`; there is no fast-mode logic change in that file.
- Clean temporary checkouts were created in:
  - `/tmp/vanilla-stwo-cairo`
  - `/tmp/vanilla-stwo`
- Commits compared:
  - `stwo-cairo`: `54915afd2085c6be1f864f941725bdfe7eb5323d`
  - `stwo`: `293f7c6ff405ec416496a12e644a196e86d12ddf`
- Both proof runs used the same input file:
  - `/tmp/vanilla-stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_ret_opcode/compiled.json`
- Local fast proof command:
  - `rtk proxy env STWO_PROVER_MEMORY_MODE=fast rustup run nightly-2025-06-20 cargo run --manifest-path /Users/lucas/stwo-cairo/stwo_cairo_prover/Cargo.toml -p stwo-cairo-dev-utils --bin run_and_prove --release -- --program /tmp/vanilla-stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_ret_opcode/compiled.json --proof_path /tmp/local-fast-proof.bin --proof-format binary`
- Vanilla fast proof command:
  - `rtk proxy env STWO_PROVER_MEMORY_MODE=fast rustup run nightly-2025-06-20 cargo run --manifest-path /tmp/vanilla-stwo-cairo/stwo_cairo_prover/Cargo.toml -p stwo-cairo-dev-utils --bin run_and_prove --release -- --program /tmp/vanilla-stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_ret_opcode/compiled.json --proof_path /tmp/vanilla-fast-proof.bin --proof-format binary`
- Verification results:
  - `cmp -s /tmp/local-fast-proof.bin /tmp/vanilla-fast-proof.bin` succeeded.
  - `shasum -a 256` for both files:
    - `99bc696a05c45d9e6db2e8ca81cc0cbdc4ec019d691a6d179c0e8ad4fd970d40`
  - Both files are `441K`.
- Conclusion:
  - on the `ret_opcode` fixture, the current local fast path produces the exact same proof bytes as a clean vanilla checkout of the same `stwo-cairo` and `stwo` commits.

# Fresh Peak Profiling Pass

## Plan

- [x] Record a fresh Allocations trace for the exact sequencer privacy-demo low-memory workload.
- [x] Inspect the resulting trace bundle and exported metadata for peak-attribution data.
- [x] Summarize the most likely next peak targets and any remaining visibility gaps.

## Review

- Recorded a fresh Instruments `Allocations` trace for the exact sequencer privacy-demo low-memory
  workload using the re-signed test binary
  `/tmp/starknet_transaction_prover-90dc1c60897b5716-profiled`.
- Trace artifacts:
  - `/tmp/allocations-sequencer-privacy-demo-low-memory-20260403.trace`
  - `/tmp/allocations-sequencer-privacy-demo-low-memory-20260403.toc.xml`
  - `/tmp/allocations-sequencer-privacy-demo-low-memory-20260403.statistics.xml`
  - `/tmp/allocations-sequencer-privacy-demo-low-memory-20260403.allocations.xml`
  - `/tmp/allocations-sequencer-privacy-demo-low-memory-20260403.stdout`
- The profiled workload passed:
  - `test proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction ... ok`
  - trace duration: `88.218191s`
  - target stdout duration: `86.37s`
- Compared with the earlier low-memory trace export, the new trace shows materially smaller malloc VM
  zones after the recursive-precompute low-memory fix:
  - `All VM Regions`: `6,780,747,776` -> `4,352,917,504` (`-2,427,830,272`, about `-2.26 GiB`)
  - `VM: MALLOC_LARGE`: `3,420,405,760` -> `1,614,413,824` (`-1,805,991,936`, about `-1.68 GiB`)
  - `VM: MALLOC_SMALL`: `3,166,699,520` -> `2,516,582,400` (`-650,117,120`, about `-620 MiB`)
  - `All Anonymous VM`: `173,195,264` -> `168,935,424` (essentially unchanged)
- The large power-of-two heap churn is still present at roughly the same totals:
  - `Malloc 32,00 MiB`: `15,133,048,832` -> `15,569,256,448`
  - `Malloc 16,00 MiB`: unchanged at `16,844,324,864`
  - `Malloc 8,00 MiB`: `5,209,325,568` -> `5,402,263,552`
  - `Malloc 4,00 MiB`: `8,111,783,936` -> `8,208,252,928`
- Interpretation:
  - the fix reduced simultaneous residency of the big STWO buffers, which is why mobile footprint
    improved by about `1.0` to `1.5 GiB`,
  - but it did not remove the underlying large buffer creation pattern,
  - so the next win still needs to come from reducing overlap or materialization in another proving
    phase rather than from allocator cleanup.
- The CLI export limitation remains:
  - `Allocations List` still exports only end-live rows,
  - the fresh export sums to `177,951,104` live bytes at process end, with only `76,845,904`
    bytes attributed to the prover binary,
  - so the exported XML still does not identify the actual high-water-mark owners.
- `VM Tracker -> Regions Map` still exports an empty XML node through `xctrace export`.
- Most likely next peak target:
  - another early STWO heap-materialization wave composed of the same `32 MiB`, `16 MiB`, `8 MiB`,
    and `4 MiB` buffers, but now outside the recursive precompute path that was already fixed.
- Remaining visibility gap:
  - to attribute the current peak to an exact call site, the next pass needs either an
    Instruments UI screenshot at the selected peak time on the fresh trace or explicit prover
    signposts around base trace, interaction trace, PCS, quotient, and FRI.

## First Peak Attribution

- User-provided Instruments screenshots for the first fresh-trace peak show a burst centered at
  about `00:16.822`.
- The stack for the dominant `Malloc 32,00 MiB`, `Malloc 16,00 MiB`, and `Malloc 2,00 MiB`
  allocations is:
  - `privacy_prove::privacy_recursive_prove`
  - `stwo_cairo_prover::prover::prove_cairo_with_precompute`
  - `stwo::prover::pcs::TreeBuilder::commit`
  - `stwo::prover::pcs::CommitmentTreeProver::new_with_memory_mode`
  - `stwo::prover::poly::circle::ops::PolyOps::evaluate_polynomials`
  - `alloc::vec`
- That means the first current peak is no longer in
  `prepare_recursive_prover_precomputes()`. It is in the first owned Cairo-proof tree build
  inside `privacy_recursive_prove()`.
- Most likely phase attribution:
  - this is the Cairo proof's first committed owned tree, so it is most likely the base-trace
    commitment inside `prove_cairo_with_precompute()`, not the borrowed preprocessed tree and not
    recursive-precompute setup.
- Root cause pattern:
  - `TreeBuilder::commit()` hands all collected columns to
    `CommitmentTreeProver::new_with_memory_mode()`,
  - `PolyOps::evaluate_polynomials()` pre-allocates buffers for every column and evaluates them as
    one bulk batch,
  - low-memory mode spills after extension, but the large eval buffers are still created
    simultaneously during the extension phase.
- Highest-ROI next optimization target:
  - reduce simultaneous column materialization during
    `CommitmentTreeProver::new_with_memory_mode()` for the Cairo proof tree build, likely by
    bounded batching or chunked extension / commitment rather than evaluating the full tree's
    columns in one shot.

# First Peak Reduction

## Plan

- [ ] Change low-memory tree extension so completed eval columns are converted to file-backed
  storage immediately instead of after the full tree is extended.
- [ ] Keep the fast path byte-for-byte unchanged and preserve low-memory proof bytes.
- [ ] Run a focused proof-equivalence test for `stwo-cairo`.
- [ ] Rebuild and rerun the exact sequencer privacy-demo low-memory workload.
- [ ] Record the new memory result and any remaining peak owner in this file.

# All Builtins Proof Identity Check

## Plan

- [x] Generate a local `all_builtins` fast-path proof from the current checkout.
- [x] Generate a local `all_builtins` low-memory proof from the current checkout.
- [x] Compare local fast and low-memory proofs byte-for-byte.
- [x] Generate a clean vanilla `all_builtins` proof at the current `stwo-cairo` and `stwo`
  commits.
- [x] Compare local and vanilla proofs byte-for-byte and record hashes.

## Review

- Local build:
  - built `run_and_prove` from the current dirty checkout with
    `CARGO_TARGET_DIR=/tmp/allbuiltins-local-build`
  - manifest:
    `/Users/lucas/stwo-cairo/stwo_cairo_prover/Cargo.toml`
- Vanilla build:
  - archived clean source snapshots for
    `stwo-cairo@54915afd2085c6be1f864f941725bdfe7eb5323d` and
    `stwo@293f7c6ff405ec416496a12e644a196e86d12ddf`
  - rewired the archived `stwo_cairo_prover/Cargo.toml` to the archived clean `stwo` path
  - built with `CARGO_TARGET_DIR=/tmp/allbuiltins-vanilla-build`
- Proof artifacts:
  - local fast:
    `/tmp/local-all-builtins-fast-proof.bin`
  - local low-memory:
    `/tmp/local-all-builtins-low-memory-proof.bin`
  - vanilla fast:
    `/tmp/vanilla-all-builtins-fast-proof.bin`
- Exact workload:
  - binary: `run_and_prove`
  - program:
    `/Users/lucas/stwo-cairo/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/compiled.json`
    for local
  - program:
    `/tmp/vanilla-stwo-cairo-54915afd2085c6be1f864f941725bdfe7eb5323d/stwo_cairo_prover/test_data/test_prove_verify_all_builtins/compiled.json`
    for vanilla
  - args:
    `--proof_path <...> --proof-format binary`
- Result:
  - local fast vs local low-memory: byte-identical
  - local fast vs vanilla fast: byte-identical
  - local low-memory vs vanilla fast: byte-identical
- Hashes and sizes:
  - SHA-256:
    `92f243997408b67cf6db12a87110288b78e6f39f50d13a9cf8333b0c3ceabe55`
  - size:
    `1,250,576` bytes

# Late Recursive Verifier Peak Reduction

## Plan

- [x] Identify the remaining late peak from the recursive verifier path.
- [x] Reduce `Vec` growth churn during recursive verifier circuit construction.
- [x] Rebuild and rerun the exact sequencer privacy-demo low-memory workload.
- [x] Re-run a smaller sequencer recursive-proof test on the patched graph in fast and low-memory
  modes.
- [ ] Re-prove byte identity for the full recursive proof on the patched graph.

## Review

- Peak attribution from Instruments:
  - the remaining late visible spike came from the recursive verifier / circuit builder path,
    with stacks through `circuits_stark_verifier`, `circuit_cairo_air::verify`, `_realloc`, and
    `alloc::raw_vec::finish_grow`.
  - that pointed to repeated capacity growth in the verifier circuit context rather than to another
    STWO commitment-tree burst.
- Implemented reduction:
  - added `ContextCapacities` / `CircuitCapacities` snapshots plus
    `Context::reserve_capacities(...)` in
    `/Users/lucas/stwo-circuits/crates/circuits/src/context.rs`
  - added `build_fixed_cairo_circuit_with_capacities(...)` in
    `/Users/lucas/stwo-circuits/crates/cairo_air/src/verify.rs`
  - updated `privacy_recursive_prove()` precompute flow in
    `/Users/lucas/proving-utils/crates/privacy_prove/src/lib.rs`
    to:
    - build the no-value cairo verifier circuit once,
    - capture its final capacities after `add_zk_blinding(...)`,
    - reuse those capacities when constructing the value-filled verifier context during proving
- Exact sequencer measurement:
  - build graph: sequencer patched to the local `stwo`, `stwo-cairo`, `stwo-circuits`, and
    `proving-utils` repos
  - binary:
    `/tmp/sequencer-late-spike-build/release/deps/starknet_transaction_prover-90dc1c60897b5716`
  - command:
    `STWO_PROVER_MEMORY_MODE=low_memory CHAIN_ID=SN_INTEGRATION_SEPOLIA /usr/bin/time -l <binary> proving::virtual_snos_prover_test::test_prove_privacy_demo_transaction --ignored --exact --nocapture`
  - result:
    - passed
    - `58.71s` real
    - `7,517,077,504` max RSS
    - `5,215,655,000` peak memory footprint
- Improvement vs the immediately previous low-memory baseline for the same optimized branch:
  - previous:
    `6,425,744,640`
  - current:
    `5,215,655,000`
  - delta:
    `-1,210,089,640` bytes, about `-1.13 GiB` (`-18.8%`)
- Additional sequencer graph verification:
  - `proving::prover_test::test_prove_cairo_pie_10_transfers` passes on the same built binary in
    both modes:
    - fast: `12.75s`
    - low-memory: `61.55s`
- Remaining verification gap:
  - the recursive path now has strong functional verification on the real sequencer graph, but I
    have not yet re-established full recursive proof byte identity after this preallocation change.
  - the existing direct `privacy-prove` byte-identity test is difficult to run outside the
    sequencer workspace because the standalone `proving-utils` workspace and the local
    `stwo-circuits` workspace otherwise pull a mixed `stwo` graph.
