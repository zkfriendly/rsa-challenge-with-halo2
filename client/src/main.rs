use std::{
    fs::{read, write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use contract_build::{
    BuildMode, ExecuteArgs, ManifestPath, OptimizationPasses, DEFAULT_MAX_MEMORY_PAGES,
};
use contract_extrinsics::{BalanceVariant, ExtrinsicOptsBuilder, InstantiateCommandBuilder};
use rsa_circuit::utils::{generate_proof, generate_setup, Account, Setup};
use subxt::{
    config::{substrate::BlakeTwo256, Hasher},
    dynamic::Value,
    ext::scale_value::Composite,
    OnlineClient, PolkadotConfig,
};

use crate::command::Command;

const CIRCUIT_MAX_K: u32 = 5;
const SNARK_SETUP_FILE: &str = "snark-setup";
const PROOF_FILE: &str = "submission-data";
const ACCOUNT: Account = [0; 32];

mod command;

fn read_setup() -> Result<Setup> {
    let setup_serialized = read(SNARK_SETUP_FILE).context("Failed to read SNARK setup")?;
    Ok(Setup::from_bytes(&mut setup_serialized.as_slice()))
}

#[tokio::main]
async fn main() -> Result<()> {
    match Command::parse() {
        Command::SetupSnark => {
            println!("⏳ Generating SNARK setup...");
            let setup = generate_setup(CIRCUIT_MAX_K);
            println!("✅ Generated SNARK setup");
            write(SNARK_SETUP_FILE, setup.to_bytes()).context("Failed to save SNARK setup")?;
            println!("💾 Saved SNARK setup to `{SNARK_SETUP_FILE}`");
        }
        Command::GenerateProof { p, q } => {
            println!("⏳ Preparing for SNARK proof generation...");
            let setup = read_setup()?;
            println!("✅ Loaded SNARK setup from `{SNARK_SETUP_FILE}`");

            println!("⏳ Generating SNARK proof...");
            let proof = generate_proof(&setup, p, q, ACCOUNT);
            println!("✅ Generated SNARK proof");
            write(PROOF_FILE, proof.to_bytes()).context("Failed to save SNARK proof")?;
            println!("💾 Saved SNARK proof to `{PROOF_FILE}`");
        }
        Command::RegisterVk => {
            println!("⏳ Preparing for verification key registration...");
            let vk_bytes = read_setup()?.serialize_vk();

            let api = OnlineClient::<PolkadotConfig>::new().await?;
            let call = subxt::dynamic::tx(
                "VkStorage",
                "store_key",
                Composite::unnamed([Value::from_bytes(&vk_bytes)]),
            );

            println!("⏳ Registering verification key...");
            api.tx()
                .sign_and_submit_then_watch_default(&call, &subxt_signer::sr25519::dev::alice())
                .await?
                .wait_for_finalized_success()
                .await?;
            println!("✅ Registered verification key");
        }
        Command::BuildContract => {
            println!("⏳ Building contract...");
            contract_build::execute(ExecuteArgs {
                manifest_path: ManifestPath::new(get_contract_manifest().into())?,
                verbosity: Default::default(),
                build_mode: BuildMode::Release,
                features: Default::default(),
                network: Default::default(),
                build_artifact: Default::default(),
                unstable_flags: Default::default(),
                optimization_passes: Some(OptimizationPasses::default()),
                keep_debug_symbols: false,
                dylint: false,
                output_type: Default::default(),
                skip_wasm_validation: false,
                target: Default::default(),
                max_memory_pages: DEFAULT_MAX_MEMORY_PAGES,
                image: Default::default(),
            })?;
            println!("✅ Contract built");
        }
        Command::DeployContract { challenge, reward } => {
            println!("⏳ Deploying contract...");
            let setup = read_setup()?;

            let vk_bytes = setup.serialize_vk();
            println!("✅ Loaded vk from `{SNARK_SETUP_FILE}`");
            let vk_hash = BlakeTwo256::hash(&vk_bytes);

            let command = InstantiateCommandBuilder::default()
                .args(vec![challenge.to_string(), format!("{vk_hash:?}")])
                .value(BalanceVariant::Default(reward))
                .extrinsic_opts(
                    ExtrinsicOptsBuilder::default()
                        .suri("//Alice")
                        .manifest_path(Some(get_contract_manifest()))
                        .done(),
                )
                .done()
                .await?;
            println!("⏳ Instantiating contract...");
            command.instantiate(None).await.unwrap();
            println!("✅ Contract deployed");
        }
        Command::SubmitSolution => {}
    }
    Ok(())
}

fn get_contract_manifest() -> impl Into<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../rsa_contract/Cargo.toml")
}
