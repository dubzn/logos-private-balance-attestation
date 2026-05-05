//! Deployable LEZ program for the balance-attestation gate.
//!
//! Reads the LEZ program input via `read_nssa_inputs::<Instruction>()`,
//! dispatches `InitGate` or `Admit`, and emits a [`ProgramOutput`] that the
//! LEZ runtime persists.
//!
//! # Trust bridge
//!
//! This program does **not** verify the outer balance-attestation receipt.
//! Spike 08 showed that the current live local sequencer accepts an
//! `Admit { outer_journal }` with no external RISC Zero receipt bound at
//! admission time. For this implementation track, the cryptographic guarantee
//! is therefore a host-side precheck: the CLI/SDK must verify the envelope (and
//! the outer gate receipt, when present) before submitting the LEZ transaction.
//!
//! Once the journal is host-verified, this program checks (a) the outer journal's
//! `inner_image_id` matches the pinned `BALANCE_ATTESTATION_ID` recorded in
//! gate state, (b) the journal's gate context matches the recomputed context
//! id, (c) the journal's threshold matches gate state, (d) the journal's
//! `accepted_presenter_id` matches `H(PRESENTER_DOMAIN || presenter_pubkey)`
//! where the pubkey is read from the authorized presenter pre-state account's
//! `data` field, (e) the nullifier has not been admitted before.

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::{Account, AccountWithMetadata};
use nssa_core::program::{
    AccountPostState, Claim, DEFAULT_PROGRAM_ID, ProgramInput, ProgramOutput, read_nssa_inputs,
};
use risc0_zkvm::sha::{Impl as Sha256Impl, Sha256};
use serde::{Deserialize, Serialize};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";
const GATE_STATE_MAGIC: [u8; 4] = *b"BAT1";
const GATE_STATE_VERSION: u16 = 1;
const OUTER_JOURNAL_VERSION: u16 = 1;
const PRESENTER_PUBKEY_LEN: usize = 32;

/// Borsh-encoded gate state held in `pre_states[0].account.data`.
///
/// Wire format must stay in lock-step with `idl/balance-attestation-verifier.json`.
#[derive(BorshSerialize, BorshDeserialize)]
struct GateState {
    magic: [u8; 4],
    version: u16,
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    expected_inner_image_id: [u8; 32],
    admitted_nullifiers: Vec<[u8; 32]>,
}

/// Outer LEZ-gate journal (matches `lez_verifier::LezGateJournal` byte-for-byte
/// when round-tripped through borsh).
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

fn main() {
    let (
        ProgramInput {
            self_program_id,
            caller_program_id,
            pre_states,
            instruction,
        },
        instruction_words,
    ) = read_nssa_inputs::<Instruction>();

    if pre_states.len() != 2 {
        panic!("BA503 InvalidGateAccount: program requires exactly 2 pre states (gate, presenter)");
    }

    let pre_states_clone = pre_states.clone();

    let post_states = match instruction {
        Instruction::RegisterPresenter { presenter_pubkey } => {
            register_presenter(self_program_id, pre_states, presenter_pubkey)
        }
        Instruction::InitGate {
            chain_id,
            verifier_id,
            gate_id,
            threshold,
            expected_inner_image_id,
        } => init_gate(
            self_program_id,
            pre_states,
            chain_id,
            verifier_id,
            gate_id,
            threshold,
            expected_inner_image_id,
        ),
        Instruction::Admit { outer_journal } => admit(self_program_id, pre_states, &outer_journal),
    };

    ProgramOutput::new(
        self_program_id,
        caller_program_id,
        instruction_words,
        pre_states_clone,
        post_states,
    )
    .write();
}

fn register_presenter(
    self_program_id: [u32; 8],
    pre_states: Vec<AccountWithMetadata>,
    presenter_pubkey: [u8; 32],
) -> Vec<AccountPostState> {
    let [presenter_pre, admin_pre] = into_two(pre_states);

    if presenter_pre.account != Account::default() {
        panic!("BA502 UnauthorizedPresenterAccount: presenter account must be uninitialized");
    }
    if !admin_pre.is_authorized {
        panic!("BA502 UnauthorizedPresenterAccount: admin pre-state must be authorized");
    }

    let data = presenter_pubkey
        .to_vec()
        .try_into()
        .unwrap_or_else(|_| panic!("BA502 UnauthorizedPresenterAccount: presenter pubkey does not fit"));

    let _ = self_program_id;
    vec![
        AccountPostState::new_claimed(
            Account {
                program_owner: presenter_pre.account.program_owner,
                balance: presenter_pre.account.balance,
                data,
                nonce: presenter_pre.account.nonce,
            },
            Claim::Authorized,
        ),
        AccountPostState::new(admin_pre.account),
    ]
}

fn init_gate(
    self_program_id: [u32; 8],
    pre_states: Vec<AccountWithMetadata>,
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    expected_inner_image_id: [u8; 32],
) -> Vec<AccountPostState> {
    let [gate_pre, admin_pre] = into_two(pre_states);

    if gate_pre.account != Account::default() {
        panic!("BA500 GateAlreadyInitialized: gate state pre-state must be a default account");
    }
    if !admin_pre.is_authorized {
        panic!("BA502 UnauthorizedPresenterAccount: admin pre-state must be authorized");
    }

    let gate_state = GateState {
        magic: GATE_STATE_MAGIC,
        version: GATE_STATE_VERSION,
        chain_id,
        verifier_id,
        gate_id,
        threshold,
        expected_inner_image_id,
        admitted_nullifiers: Vec::new(),
    };

    let initialized = build_gate_account(&gate_state, &gate_pre.account);

    // Leave program_owner as DEFAULT_PROGRAM_ID (matches pre-state) and let
    // claim resolution flip ownership to `self_program_id` after
    // `validate_execution` passes — see `logos/programs/token/src/initialize.rs`
    // for the canonical pattern. Returning an account with program_owner
    // already set to `self_program_id` would trip rule 4 of
    // `nssa_core::program::validate_execution` (Unallowed modification of
    // program owner).
    let _ = self_program_id;
    vec![
        AccountPostState::new_claimed(initialized, Claim::Authorized),
        AccountPostState::new(admin_pre.account),
    ]
}

fn admit(
    self_program_id: [u32; 8],
    pre_states: Vec<AccountWithMetadata>,
    outer_journal_bytes: &[u8],
) -> Vec<AccountPostState> {
    let [gate_pre, presenter_pre] = into_two(pre_states);

    if gate_pre.account.program_owner == DEFAULT_PROGRAM_ID {
        panic!("BA501 GateNotInitialized: gate state has default program owner");
    }
    if gate_pre.account.program_owner != self_program_id {
        panic!("BA503 InvalidGateAccount: gate state owned by a different program");
    }

    let mut gate_state: GateState = BorshDeserialize::try_from_slice(gate_pre.account.data.as_ref())
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state borsh decode failed"));

    if gate_state.magic != GATE_STATE_MAGIC {
        panic!("BA503 InvalidGateAccount: gate state magic mismatch");
    }
    if gate_state.version != GATE_STATE_VERSION {
        panic!("BA503 InvalidGateAccount: gate state version mismatch");
    }

    let outer: OuterJournal = BorshDeserialize::try_from_slice(outer_journal_bytes)
        .unwrap_or_else(|_| panic!("BA200 InvalidReceipt: outer journal borsh decode failed"));

    if outer.version != OUTER_JOURNAL_VERSION {
        panic!("BA100 InvalidEnvelopeVersion: outer journal version unsupported");
    }
    if outer.inner_image_id != gate_state.expected_inner_image_id {
        panic!("BA102 InvalidImageId: inner image id does not match gate state");
    }

    let expected_context_id = derive_context_id(
        &gate_state.chain_id,
        &gate_state.expected_inner_image_id,
        &gate_state.verifier_id,
        &gate_state.gate_id,
        gate_state.threshold,
    );
    if outer.gate_context_id != expected_context_id {
        panic!("BA202 ContextMismatch: outer journal context does not match gate state");
    }

    if outer.accepted_threshold != gate_state.threshold {
        panic!("BA201 ThresholdMismatch: outer journal threshold does not match gate state");
    }

    if !presenter_pre.is_authorized {
        panic!("BA502 UnauthorizedPresenterAccount: presenter pre-state must be authorized");
    }
    if presenter_pre.account.program_owner != self_program_id {
        panic!("BA502 UnauthorizedPresenterAccount: presenter account must be registered with this program");
    }

    let presenter_pubkey = presenter_pubkey_from_data(presenter_pre.account.data.as_ref());
    let expected_presenter_id = derive_presenter_id(&presenter_pubkey);
    if outer.accepted_presenter_id != expected_presenter_id {
        panic!("BA203 PresenterMismatch: outer presenter id does not match presenter account");
    }

    if gate_state
        .admitted_nullifiers
        .iter()
        .any(|n| n == &outer.accepted_context_nullifier)
    {
        panic!("BA206 DuplicateNullifier: nullifier already admitted");
    }

    gate_state
        .admitted_nullifiers
        .push(outer.accepted_context_nullifier);

    let _ = self_program_id;
    let updated = build_gate_account(&gate_state, &gate_pre.account);

    vec![
        AccountPostState::new(updated),
        AccountPostState::new(presenter_pre.account),
    ]
}

fn build_gate_account(gate_state: &GateState, pre_account: &Account) -> Account {
    let bytes = borsh::to_vec(gate_state)
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state borsh encode failed"));
    let data = bytes
        .try_into()
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: gate state exceeds data max length"));

    // Preserve `program_owner` from the pre-state. For `InitGate` this stays
    // `DEFAULT_PROGRAM_ID` and the `Claim::Authorized` flip happens during
    // claim resolution; for `Admit` the pre-state is already owned by us
    // post-init, so this is `self_program_id`.
    Account {
        program_owner: pre_account.program_owner,
        balance: pre_account.balance,
        data,
        nonce: pre_account.nonce,
    }
}

fn into_two(pre_states: Vec<AccountWithMetadata>) -> [AccountWithMetadata; 2] {
    pre_states
        .try_into()
        .unwrap_or_else(|_| panic!("BA503 InvalidGateAccount: program requires exactly 2 pre states"))
}

fn presenter_pubkey_from_data(data: &[u8]) -> [u8; 32] {
    if data.len() < PRESENTER_PUBKEY_LEN {
        panic!("BA502 UnauthorizedPresenterAccount: presenter account data must hold at least 32 bytes");
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&data[..PRESENTER_PUBKEY_LEN]);
    out
}

fn derive_context_id(
    chain_id: &[u8; 32],
    circuit_image_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    threshold: u128,
) -> [u8; 32] {
    hash_segments(&[
        CONTEXT_DOMAIN,
        chain_id,
        circuit_image_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
    ])
}

fn derive_presenter_id(pubkey: &[u8; 32]) -> [u8; 32] {
    hash_segments(&[PRESENTER_DOMAIN, pubkey])
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    // Mirrors `attestation_core::hash::hash_segments` byte-for-byte: each
    // segment is prefixed by its little-endian u64 length, then the bytes
    // are concatenated and SHA-256 hashed via the risc0 accelerator. Do not
    // use `sha2` here — the guest must use the on-chip accelerator.
    let mut buf = Vec::with_capacity(
        segments
            .iter()
            .map(|s| 8 + s.len())
            .sum::<usize>(),
    );
    for seg in segments {
        buf.extend_from_slice(&(seg.len() as u64).to_le_bytes());
        buf.extend_from_slice(seg);
    }
    let digest = Sha256Impl::hash_bytes(&buf);
    let mut out = [0_u8; 32];
    out.copy_from_slice(digest.as_bytes());
    out
}
