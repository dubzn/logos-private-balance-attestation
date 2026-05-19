use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::{Account, AccountWithMetadata};
use nssa_core::program::{
    AccountPostState, Claim, DEFAULT_PROGRAM_ID, ProgramInput, ProgramOutput, read_nssa_inputs,
};
use risc0_zkvm::sha::{Impl as Sha256Impl, Sha256};
use serde::{Deserialize, Serialize};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const PPE_NULLIFIER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/ppe-nullifier";
const GATE_STATE_MAGIC: [u8; 4] = *b"BAP1";
const GATE_STATE_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PpeBalanceGateInstruction {
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct GateAdmission {
    context_nullifier: [u8; 32],
    presenter_account_id: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize)]
struct GateState {
    magic: [u8; 4],
    version: u16,
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    admissions: Vec<GateAdmission>,
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
    ) = read_nssa_inputs::<PpeBalanceGateInstruction>();

    let [holder_pre, gate_pre, presenter_pre] =
        pre_states.clone().try_into().unwrap_or_else(|_| {
            panic!("BA503 InvalidGateAccount: PPE gate expects holder, gate, presenter")
        });

    assert!(
        holder_pre.is_authorized,
        "BA502 UnauthorizedPresenterAccount: private holder must be authorized"
    );
    assert!(
        presenter_pre.is_authorized,
        "BA502 UnauthorizedPresenterAccount: presenter account must be authorized"
    );
    assert!(
        holder_pre.account.balance >= instruction.threshold,
        "BA201 ThresholdMismatch: private balance is below threshold"
    );

    let context_id = derive_context_id(
        &instruction.chain_id,
        &instruction.verifier_id,
        &instruction.gate_id,
        instruction.threshold,
    );
    let context_nullifier = derive_context_nullifier(holder_pre.account_id.value(), &context_id);

    let mut gate_state = load_or_init_gate_state(self_program_id, &gate_pre, &instruction);
    assert_gate_matches_instruction(&gate_state, &instruction);

    if gate_state
        .admissions
        .iter()
        .any(|admission| admission.context_nullifier == context_nullifier)
    {
        panic!("BA206 DuplicateNullifier: nullifier already admitted");
    }

    gate_state.admissions.push(GateAdmission {
        context_nullifier,
        presenter_account_id: presenter_pre.account_id.into_value(),
    });

    let holder_post = AccountPostState::new(holder_pre.account);
    let gate_post = write_gate_post_state(&gate_state, &gate_pre.account);
    let presenter_post = AccountPostState::new(presenter_pre.account);

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_data,
        pre_states,
        vec![holder_post, gate_post, presenter_post],
    )
    .write();
}

fn load_or_init_gate_state(
    self_program_id: [u32; 8],
    gate_pre: &AccountWithMetadata,
    instruction: &PpeBalanceGateInstruction,
) -> GateState {
    if gate_pre.account == Account::default() {
        if !gate_pre.is_authorized {
            panic!("BA503 InvalidGateAccount: default gate account must be authorized for claim");
        }

        return GateState {
            magic: GATE_STATE_MAGIC,
            version: GATE_STATE_VERSION,
            chain_id: instruction.chain_id,
            verifier_id: instruction.verifier_id,
            gate_id: instruction.gate_id,
            threshold: instruction.threshold,
            admissions: Vec::new(),
        };
    }

    if gate_pre.account.program_owner == DEFAULT_PROGRAM_ID {
        panic!("BA501 GateNotInitialized: non-default gate account has default owner");
    }
    if gate_pre.account.program_owner != self_program_id {
        panic!("BA503 InvalidGateAccount: gate account is owned by a different program");
    }

    let state: GateState = BorshDeserialize::try_from_slice(gate_pre.account.data.as_ref())
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state borsh decode failed"));
    if state.magic != GATE_STATE_MAGIC {
        panic!("BA503 InvalidGateAccount: gate state magic mismatch");
    }
    if state.version != GATE_STATE_VERSION {
        panic!("BA503 InvalidGateAccount: gate state version mismatch");
    }
    state
}

fn assert_gate_matches_instruction(
    gate_state: &GateState,
    instruction: &PpeBalanceGateInstruction,
) {
    if gate_state.chain_id != instruction.chain_id
        || gate_state.verifier_id != instruction.verifier_id
        || gate_state.gate_id != instruction.gate_id
    {
        panic!("BA202 ContextMismatch: gate context does not match instruction");
    }
    if gate_state.threshold != instruction.threshold {
        panic!("BA201 ThresholdMismatch: gate threshold does not match instruction");
    }
}

fn write_gate_post_state(gate_state: &GateState, pre_account: &Account) -> AccountPostState {
    let bytes = borsh::to_vec(gate_state)
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state borsh encode failed"));
    let data = bytes
        .try_into()
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state exceeds data max length"));
    let account = Account {
        program_owner: pre_account.program_owner,
        balance: pre_account.balance,
        data,
        nonce: pre_account.nonce,
    };
    AccountPostState::new_claimed_if_default(account, Claim::Authorized)
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
    let mut buf = Vec::with_capacity(
        segments
            .iter()
            .map(|segment| 8 + segment.len())
            .sum::<usize>(),
    );
    for segment in segments {
        buf.extend_from_slice(&(segment.len() as u64).to_le_bytes());
        buf.extend_from_slice(segment);
    }

    let digest = Sha256Impl::hash_bytes(&buf);
    let mut out = [0_u8; 32];
    out.copy_from_slice(digest.as_bytes());
    out
}
