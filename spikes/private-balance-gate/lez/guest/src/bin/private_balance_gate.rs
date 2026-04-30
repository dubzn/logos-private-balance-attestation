use nssa_core::program::{AccountPostState, Claim, ProgramInput, ProgramOutput, read_nssa_inputs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateBalanceGateInstruction {
    threshold: u128,
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
    ) = read_nssa_inputs::<PrivateBalanceGateInstruction>();

    let [holder_pre, gate_pre] = pre_states
        .clone()
        .try_into()
        .unwrap_or_else(|_| panic!("private balance gate expects holder and gate accounts"));

    assert!(
        holder_pre.is_authorized,
        "private holder account must be authorized"
    );
    assert!(gate_pre.is_authorized, "public gate account must be authorized");
    assert!(
        holder_pre.account.balance >= instruction.threshold,
        "private balance is below threshold"
    );

    let holder_post = AccountPostState::new(holder_pre.account.clone());

    let mut gate_account = gate_pre.account.clone();
    let mut gate_data = gate_account.data.into_inner();
    gate_data.extend_from_slice(&instruction.marker);
    gate_account.data = gate_data
        .try_into()
        .expect("private balance gate marker should fit account data limits");

    let gate_post = AccountPostState::new_claimed_if_default(gate_account, Claim::Authorized);

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_data,
        pre_states,
        vec![holder_post, gate_post],
    )
    .write();
}

