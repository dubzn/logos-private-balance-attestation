use nssa::{AccountId, program::Program};
use serde::{Deserialize, Serialize};
use wallet::{PrivacyPreservingAccount, WalletCore};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateBalanceGateInstruction {
    threshold: u128,
    marker: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let program_path = std::env::args_os().nth(1).unwrap().into_string().unwrap();
    let private_account_id: AccountId = std::env::args_os()
        .nth(2)
        .unwrap()
        .into_string()
        .unwrap()
        .parse()
        .unwrap();
    let gate_account_id: AccountId = std::env::args_os()
        .nth(3)
        .unwrap()
        .into_string()
        .unwrap()
        .parse()
        .unwrap();
    let threshold: u128 = std::env::args_os()
        .nth(4)
        .unwrap()
        .into_string()
        .unwrap()
        .parse()
        .unwrap();
    let marker = std::env::args()
        .nth(5)
        .unwrap_or_else(|| "private-balance-gate-ok".to_string())
        .into_bytes();

    let bytecode = std::fs::read(program_path).unwrap();
    let program = Program::new(bytecode).unwrap();
    let wallet_core = WalletCore::from_env().unwrap();

    let instruction = PrivateBalanceGateInstruction { threshold, marker };
    let accounts = vec![
        PrivacyPreservingAccount::PrivateOwned(private_account_id),
        PrivacyPreservingAccount::Public(gate_account_id),
    ];

    let (hash, _shared_secrets) = wallet_core
        .send_privacy_preserving_tx(
            accounts,
            Program::serialize_instruction(instruction).unwrap(),
            &program.into(),
        )
        .await
        .unwrap();

    println!("{hash:?}");
}

