//! Spike 08 — submit our deployable LEZ program against a live local sequencer
//! and observe whether the LEZ submission path binds the outer balance-attestation
//! receipt (Best), accepts only the program execution (Workable), or rejects
//! the transaction outright (Blocked).
//!
//! Usage (typically driven by `scripts/spike-08-run.sh`):
//!
//! ```text
//! spike_08_run \
//!   --program-bin <bin>          # ELF produced by lez-verifier-program
//!   --gate-account <Public/...>   # uninitialized gate-state account
//!   --admin-account <Public/...>  # admin signer for register/init
//!   --presenter-account <Public/...> # presenter account (signs the tx)
//!   --presenter-pubkey-hex <hex64>   # 32-byte BIP-340 x-only Schnorr pubkey
//!   --chain-id-hex <hex64>
//!   --verifier-id-hex <hex64>
//!   --gate-id-hex <hex64>
//!   --threshold <u128>
//!   --inner-image-id-hex <hex64>     # the BALANCE_ATTESTATION_ID (inner circuit)
//!   --nullifier-hex <hex64>          # context_nullifier from the real envelope
//!   --presenter-id-hex <hex64>       # H(PRESENTER_DOMAIN || pubkey) from the envelope
//!   [--mode register-presenter|init-gate|admit|admit-fabricated]
//! ```
//!
//! `init-gate` uses [`Instruction::InitGate`]; `admit` uses [`Instruction::Admit`]
//! with a journal assembled from CLI fields that are expected to have come from
//! a host-verified envelope; `admit-fabricated` submits the same wire shape with
//! caller-supplied arbitrary values. This runner deliberately does not consume a
//! receipt or envelope: it isolates whether the live sequencer/program path
//! itself rejects forged journals.

use anyhow::{anyhow, bail, Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use common::transaction::LeeTransaction;
use lee::{program::Program, AccountId};
use sequencer_service_rpc::RpcClient as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use wallet::WalletCore;

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";

#[derive(BorshSerialize, BorshDeserialize)]
struct OuterJournal {
    version: u16,
    inner_image_id: [u8; 32],
    gate_context_id: [u8; 32],
    accepted_context_nullifier: [u8; 32],
    accepted_presenter_id: [u8; 32],
    accepted_threshold: u128,
}

#[derive(Serialize, Deserialize)]
enum Instruction {
    RegisterPresenter {
        presenter_pubkey: [u8; 32],
    },
    InitGate {
        chain_id: [u8; 32],
        verifier_id: [u8; 32],
        gate_id: [u8; 32],
        threshold: u128,
        expected_inner_image_id: [u8; 32],
    },
    Admit {
        outer_journal: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    RegisterPresenter,
    InitGate,
    Admit,
    AdmitFabricated,
}

impl Mode {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "register-presenter" => Ok(Self::RegisterPresenter),
            "init-gate" => Ok(Self::InitGate),
            "admit" => Ok(Self::Admit),
            "admit-fabricated" => Ok(Self::AdmitFabricated),
            other => bail!("unknown --mode value: {other}"),
        }
    }
}

struct Args {
    program_bin: PathBuf,
    gate_account: AccountId,
    admin_account: Option<AccountId>,
    presenter_account: AccountId,
    presenter_pubkey: [u8; 32],
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    inner_image_id: [u8; 32],
    nullifier: [u8; 32],
    presenter_id: [u8; 32],
    mode: Mode,
}

fn parse_args() -> Result<Args> {
    let mut iter = std::env::args().skip(1);
    let mut program_bin: Option<PathBuf> = None;
    let mut gate_account: Option<String> = None;
    let mut admin_account: Option<String> = None;
    let mut presenter_account: Option<String> = None;
    let mut presenter_pubkey_hex: Option<String> = None;
    let mut chain_id_hex: Option<String> = None;
    let mut verifier_id_hex: Option<String> = None;
    let mut gate_id_hex: Option<String> = None;
    let mut threshold: Option<u128> = None;
    let mut inner_image_id_hex: Option<String> = None;
    let mut nullifier_hex: Option<String> = None;
    let mut presenter_id_hex: Option<String> = None;
    let mut mode: Mode = Mode::Admit;

    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| anyhow!("missing value for {flag}"))
        };
        match flag.as_str() {
            "--program-bin" => program_bin = Some(PathBuf::from(value()?)),
            "--gate-account" => gate_account = Some(value()?),
            "--admin-account" => admin_account = Some(value()?),
            "--presenter-account" => presenter_account = Some(value()?),
            "--presenter-pubkey-hex" => presenter_pubkey_hex = Some(value()?),
            "--chain-id-hex" => chain_id_hex = Some(value()?),
            "--verifier-id-hex" => verifier_id_hex = Some(value()?),
            "--gate-id-hex" => gate_id_hex = Some(value()?),
            "--threshold" => threshold = Some(value()?.parse()?),
            "--inner-image-id-hex" => inner_image_id_hex = Some(value()?),
            "--nullifier-hex" => nullifier_hex = Some(value()?),
            "--presenter-id-hex" => presenter_id_hex = Some(value()?),
            "--mode" => mode = Mode::parse(&value()?)?,
            other => bail!("unknown flag: {other}"),
        }
    }

    Ok(Args {
        program_bin: program_bin.context("missing --program-bin")?,
        gate_account: gate_account
            .context("missing --gate-account")?
            .strip_prefix("Public/")
            .context("--gate-account must be Public/<id>")?
            .parse()?,
        admin_account: match admin_account {
            Some(account) => Some(
                account
                    .strip_prefix("Public/")
                    .context("--admin-account must be Public/<id>")?
                    .parse()?,
            ),
            None => None,
        },
        presenter_account: presenter_account
            .context("missing --presenter-account")?
            .strip_prefix("Public/")
            .context("--presenter-account must be Public/<id>")?
            .parse()?,
        presenter_pubkey: hex_to_array(
            &presenter_pubkey_hex.context("missing --presenter-pubkey-hex")?,
        )?,
        chain_id: hex_to_array(&chain_id_hex.context("missing --chain-id-hex")?)?,
        verifier_id: hex_to_array(&verifier_id_hex.context("missing --verifier-id-hex")?)?,
        gate_id: hex_to_array(&gate_id_hex.context("missing --gate-id-hex")?)?,
        threshold: threshold.context("missing --threshold")?,
        inner_image_id: hex_to_array(&inner_image_id_hex.context("missing --inner-image-id-hex")?)?,
        nullifier: hex_to_array(&nullifier_hex.context("missing --nullifier-hex")?)?,
        presenter_id: hex_to_array(&presenter_id_hex.context("missing --presenter-id-hex")?)?,
        mode,
    })
}

fn hex_to_array(s: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(s.trim_start_matches("0x"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("expected 32-byte hex, got {} bytes", bytes.len()))
}

fn derive_context_id(
    chain_id: &[u8; 32],
    inner_image_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    threshold: u128,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let segments: [&[u8]; 6] = [
        CONTEXT_DOMAIN,
        chain_id,
        inner_image_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
    ];
    for seg in segments {
        hasher.update((seg.len() as u64).to_le_bytes());
        hasher.update(seg);
    }
    hasher.finalize().into()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let bytecode = std::fs::read(&args.program_bin)
        .with_context(|| format!("read program bin {}", args.program_bin.display()))?;
    let program = Program::new(bytecode.into()).context("decode program ELF")?;
    eprintln!("program_id (pinned) = {:?}", program.id());

    let wallet_core = WalletCore::from_env().context("WalletCore::from_env")?;

    let instruction = match args.mode {
        Mode::RegisterPresenter => Instruction::RegisterPresenter {
            presenter_pubkey: args.presenter_pubkey,
        },
        Mode::InitGate => Instruction::InitGate {
            chain_id: args.chain_id,
            verifier_id: args.verifier_id,
            gate_id: args.gate_id,
            threshold: args.threshold,
            expected_inner_image_id: args.inner_image_id,
        },
        Mode::Admit => {
            let context_id = derive_context_id(
                &args.chain_id,
                &args.inner_image_id,
                &args.verifier_id,
                &args.gate_id,
                args.threshold,
            );
            let journal = OuterJournal {
                version: 1,
                inner_image_id: args.inner_image_id,
                gate_context_id: context_id,
                accepted_context_nullifier: args.nullifier,
                accepted_presenter_id: args.presenter_id,
                accepted_threshold: args.threshold,
            };
            Instruction::Admit {
                outer_journal: borsh::to_vec(&journal)?,
            }
        }
        Mode::AdmitFabricated => {
            // Same shape as Admit but the caller supplies values that DO NOT
            // need to correspond to a real outer balance-attestation receipt.
            // If the sequencer applies this transaction, it confirms the LEZ
            // submission path does not bind external receipts by itself — i.e.
            // the host trust seat documented in `lez-verifier/program/README.md`
            // is the only thing standing between us and forged admissions.
            let context_id = derive_context_id(
                &args.chain_id,
                &args.inner_image_id,
                &args.verifier_id,
                &args.gate_id,
                args.threshold,
            );
            let journal = OuterJournal {
                version: 1,
                inner_image_id: args.inner_image_id,
                gate_context_id: context_id,
                accepted_context_nullifier: args.nullifier,
                accepted_presenter_id: args.presenter_id,
                accepted_threshold: args.threshold,
            };
            Instruction::Admit {
                outer_journal: borsh::to_vec(&journal)?,
            }
        }
    };

    let account_ids = match args.mode {
        // RegisterPresenter expects pre_states[0] = presenter account and
        // pre_states[1] = an authorized admin.
        Mode::RegisterPresenter => vec![args.presenter_account, required_admin(&args)?],
        Mode::InitGate => vec![args.gate_account, required_admin(&args)?],
        Mode::Admit | Mode::AdmitFabricated => vec![args.gate_account, args.presenter_account],
    };
    let nonces = wallet_core
        .get_accounts_nonces(account_ids.clone())
        .await
        .context("get_accounts_nonces")?;
    let message = lee::public_transaction::Message::try_new(
        program.id(),
        account_ids.clone(),
        nonces,
        instruction,
    )
    .context("Message::try_new")?;

    let signing_keys = account_ids
        .iter()
        .map(|account_id| {
            wallet_core
                .get_account_public_signing_key(*account_id)
                .cloned()
                .with_context(|| format!("missing signing key for account {account_id:?}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let signing_key_refs = signing_keys.iter().collect::<Vec<_>>();
    let witness_set =
        lee::public_transaction::WitnessSet::for_message(&message, &signing_key_refs);
    let tx = lee::PublicTransaction::new(message, witness_set);

    let hash = wallet_core
        .sequencer_client
        .send_transaction(LeeTransaction::Public(tx))
        .await
        .context("sequencer_client.send_transaction")?;

    println!("submitted_tx_hash={hash:?}");
    println!("mode={:?}", args.mode);
    println!("presenter_pubkey={}", hex::encode(args.presenter_pubkey));
    Ok(())
}

fn required_admin(args: &Args) -> Result<AccountId> {
    args.admin_account
        .ok_or_else(|| anyhow!("--admin-account is required for {:?}", args.mode))
}
