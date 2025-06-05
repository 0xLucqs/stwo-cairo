use cairo_air::CairoProof;
pub use cairo_lang_runner::Arg;
use prove::prover_input_from_runner;
use stwo_cairo_prover::stwo_prover::core::pcs::PcsConfig;
use stwo_cairo_prover::stwo_prover::core::vcs::blake2_merkle::Blake2sMerkleHasher;

pub mod args;
pub mod execute;
pub mod prove;

pub fn execute_and_prove(
    target_path: &str,
    args: Vec<Arg>,
    pcs_config: PcsConfig,
) -> CairoProof<Blake2sMerkleHasher> {
    // Execute.
    let executable = serde_json::from_reader(std::fs::File::open(target_path).unwrap())
        .expect("Failed to read executable");
    let runner = execute::execute(executable, args);

    // Prove.
    let prover_input = prover_input_from_runner(&runner);
    prove::prove(prover_input, pcs_config)
}
