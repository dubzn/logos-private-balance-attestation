use std::convert::Infallible;

use nssa_core::program::{AccountPostState, Claim, ProgramInput, ProgramOutput, read_nssa_inputs};
use serde::{Deserialize, Serialize};

const TOY_IMAGE_ID_HEX: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReceiptGateInstruction {
    expected_journal_words: Vec<u32>,
    marker: Vec<u8>,
}

fn main() {
    let (
        ProgramInput {
            self_program_id,
            caller_program_id,
            pre_states,
            instruction,
        },
        instruction_data,
    ) = read_nssa_inputs::<ReceiptGateInstruction>();

    let [pre_state] = pre_states
        .try_into()
        .unwrap_or_else(|_| panic!("receipt gate expects one account"));

    risc0_zkvm::guest::env::verify(toy_image_id(), &instruction.expected_journal_words)
        .unwrap_or_else(|_: Infallible| unreachable!("RISC Zero verify error is infallible"));

    let mut post_account = pre_state.account.clone();
    let mut data = post_account.data.into_inner();
    data.extend_from_slice(&instruction.marker);
    post_account.data = data
        .try_into()
        .expect("receipt gate marker should fit account data limits");

    let post_state = AccountPostState::new_claimed_if_default(post_account, Claim::Authorized);

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_data,
        vec![pre_state],
        vec![post_state],
    )
    .write();
}

fn toy_image_id() -> [u32; 8] {
    let bytes: [u8; 32] = hex::decode(TOY_IMAGE_ID_HEX)
        .expect("toy image id hex should decode")
        .try_into()
        .expect("toy image id must be 32 bytes");
    bytemuck::cast(bytes)
}

