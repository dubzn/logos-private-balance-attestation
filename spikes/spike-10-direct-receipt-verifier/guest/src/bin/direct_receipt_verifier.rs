#![no_main]

use borsh::BorshDeserialize;
use risc0_zkvm::{guest::env, Receipt};

risc0_zkvm::guest::entry!(main);

fn main() {
    let receipt_bytes: Vec<u8> = env::read();
    let expected_image_id: [u32; 8] = env::read();
    let expected_journal: Vec<u8> = env::read();

    let receipt = Receipt::try_from_slice(&receipt_bytes)
        .expect("S10 receipt bytes should decode with borsh");
    receipt
        .verify(expected_image_id)
        .expect("S10 direct cryptographic receipt verification failed");
    assert_eq!(
        receipt.journal.bytes, expected_journal,
        "S10 verified journal does not match caller expectation"
    );

    env::commit(&true);
}

