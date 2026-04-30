use common::transaction::NSSATransaction;
use nssa::{
    AccountId, PublicTransaction,
    program::Program,
    public_transaction::{Message, WitnessSet},
};
use sequencer_service_rpc::RpcClient as _;
use serde::{Deserialize, Serialize};
use wallet::WalletCore;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReceiptGateInstruction {
    expected_journal_words: Vec<u32>,
    marker: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let program_path = std::env::args_os().nth(1).unwrap().into_string().unwrap();
    let account_id: AccountId = std::env::args_os()
        .nth(2)
        .unwrap()
        .into_string()
        .unwrap()
        .parse()
        .unwrap();
    let marker = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "receipt-gate-ok".to_string())
        .into_bytes();

    let bytecode = std::fs::read(program_path).unwrap();
    let program = Program::new(bytecode).unwrap();
    let wallet_core = WalletCore::from_env().unwrap();

    let signing_key = wallet_core
        .storage()
        .user_data
        .get_pub_account_signing_key(account_id)
        .expect("spike account should be a local public account");

    let nonces = wallet_core
        .get_accounts_nonces(vec![account_id])
        .await
        .expect("node should be reachable to query account nonce");

    let instruction = ReceiptGateInstruction {
        expected_journal_words: Vec::new(),
        marker,
    };
    let message = Message::try_new(program.id(), vec![account_id], nonces, instruction).unwrap();
    let witness_set = WitnessSet::for_message(&message, &[signing_key]);
    let tx = PublicTransaction::new(message, witness_set);

    let response = wallet_core
        .sequencer_client
        .send_transaction(NSSATransaction::Public(tx))
        .await
        .unwrap();

    println!("{response:?}");
}

