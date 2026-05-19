use anyhow::{Context, Result, anyhow, bail};
use nssa::{AccountId, program::Program};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use wallet::{PrivacyPreservingAccount, WalletCore};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const PPE_NULLIFIER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/ppe-nullifier";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PpeBalanceGateInstruction {
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
}

struct Args {
    program_bin: PathBuf,
    private_account: AccountId,
    gate_account: AccountId,
    presenter_account: AccountId,
    threshold: u128,
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let bytecode = std::fs::read(&args.program_bin)
        .with_context(|| format!("read program bin {}", args.program_bin.display()))?;
    let program = Program::new(bytecode).context("decode program ELF")?;
    let program_id = program.id();
    let wallet_core = WalletCore::from_env().context("WalletCore::from_env")?;

    let instruction = PpeBalanceGateInstruction {
        chain_id: args.chain_id,
        verifier_id: args.verifier_id,
        gate_id: args.gate_id,
        threshold: args.threshold,
    };
    let accounts = vec![
        PrivacyPreservingAccount::PrivateOwned(args.private_account),
        PrivacyPreservingAccount::Public(args.gate_account),
        PrivacyPreservingAccount::Public(args.presenter_account),
    ];

    let (hash, _shared_secrets) = wallet_core
        .send_privacy_preserving_tx(
            accounts,
            Program::serialize_instruction(instruction).unwrap(),
            &program.into(),
        )
        .await
        .context("send_privacy_preserving_tx")?;

    println!("submitted_tx_hash={hash:?}");
    println!("program_id={program_id:?}");
    println!("mode=ppe-admit");
    println!(
        "context_nullifier={}",
        hex::encode(derive_context_nullifier(
            args.private_account.value(),
            &derive_context_id(
                &args.chain_id,
                &args.verifier_id,
                &args.gate_id,
                args.threshold,
            ),
        ))
    );
    Ok(())
}

fn parse_args() -> Result<Args> {
    let mut iter = std::env::args().skip(1);
    let mut program_bin = None;
    let mut private_account = None;
    let mut gate_account = None;
    let mut presenter_account = None;
    let mut threshold = None;
    let mut chain_id_hex = None;
    let mut verifier_id_hex = None;
    let mut gate_id_hex = None;

    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| anyhow!("missing value for {flag}"))
        };
        match flag.as_str() {
            "--program-bin" => program_bin = Some(PathBuf::from(value()?)),
            "--private-account" => private_account = Some(parse_account(&value()?, "Private")?),
            "--gate-account" => gate_account = Some(parse_account(&value()?, "Public")?),
            "--presenter-account" => presenter_account = Some(parse_account(&value()?, "Public")?),
            "--threshold" => threshold = Some(value()?.parse()?),
            "--chain-id-hex" => chain_id_hex = Some(value()?),
            "--verifier-id-hex" => verifier_id_hex = Some(value()?),
            "--gate-id-hex" => gate_id_hex = Some(value()?),
            other => bail!("unknown flag: {other}"),
        }
    }

    Ok(Args {
        program_bin: program_bin.context("missing --program-bin")?,
        private_account: private_account.context("missing --private-account")?,
        gate_account: gate_account.context("missing --gate-account")?,
        presenter_account: presenter_account.context("missing --presenter-account")?,
        threshold: threshold.context("missing --threshold")?,
        chain_id: hex_to_array(&chain_id_hex.context("missing --chain-id-hex")?)?,
        verifier_id: hex_to_array(&verifier_id_hex.context("missing --verifier-id-hex")?)?,
        gate_id: hex_to_array(&gate_id_hex.context("missing --gate-id-hex")?)?,
    })
}

fn parse_account(value: &str, prefix: &str) -> Result<AccountId> {
    value
        .strip_prefix(&format!("{prefix}/"))
        .unwrap_or(value)
        .parse()
        .with_context(|| format!("invalid {prefix} account id"))
}

fn hex_to_array(value: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(value.trim_start_matches("0x"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("expected 32-byte hex, got {} bytes", bytes.len()))
}

fn derive_context_id(
    chain_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    threshold: u128,
) -> [u8; 32] {
    hash_segments(&[
        CONTEXT_DOMAIN,
        chain_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
    ])
}

fn derive_context_nullifier(holder_account_id: &[u8; 32], context_id: &[u8; 32]) -> [u8; 32] {
    hash_segments(&[PPE_NULLIFIER_DOMAIN, holder_account_id, context_id])
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for segment in segments {
        hasher.update((segment.len() as u64).to_le_bytes());
        hasher.update(segment);
    }
    hasher.finalize().into()
}
