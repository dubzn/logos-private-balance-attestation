//! End-to-end roundtrip for the deployable LEZ balance-attestation program.
//!
//! Drives the guest via `default_executor().execute(...)` with hand-crafted
//! `pre_states` and instructions, then decodes the `ProgramOutput` and
//! verifies the resulting gate state. Run with:
//!
//! ```sh
//! cargo test --manifest-path lez-verifier/program/Cargo.toml
//! ```
//!
//! Requires the LEZ checkout at `<repo-root>/logos/` (path dep `nssa_core`).

use borsh::{BorshDeserialize, BorshSerialize};
use lez_verifier_program::{BALANCE_ATTESTATION_PROGRAM_ELF, BALANCE_ATTESTATION_PROGRAM_ID};
use nssa_core::account::{Account, AccountId, AccountWithMetadata};
use nssa_core::program::{Claim, ProgramId, ProgramOutput};
use risc0_zkvm::{default_executor, serde::to_vec, ExecutorEnv};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";
const GATE_STATE_MAGIC: [u8; 4] = *b"BAT1";

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
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

struct Fixture {
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    expected_inner_image_id: [u8; 32],
    presenter_pubkey: [u8; 32],
    nullifier: [u8; 32],
    gate_account_id: AccountId,
    presenter_account_id: AccountId,
}

impl Fixture {
    fn default_with_nullifier(nullifier: [u8; 32]) -> Self {
        Self {
            chain_id: [0x11; 32],
            verifier_id: [0x22; 32],
            gate_id: [0x33; 32],
            threshold: 25,
            expected_inner_image_id: [0x44; 32],
            presenter_pubkey: [0x55; 32],
            nullifier,
            gate_account_id: AccountId::new([0x66; 32]),
            presenter_account_id: AccountId::new([0x77; 32]),
        }
    }

    fn context_id(&self) -> [u8; 32] {
        hash_segments(&[
            CONTEXT_DOMAIN,
            &self.chain_id,
            &self.expected_inner_image_id,
            &self.verifier_id,
            &self.gate_id,
            &self.threshold.to_le_bytes(),
        ])
    }

    fn presenter_id(&self) -> [u8; 32] {
        hash_segments(&[PRESENTER_DOMAIN, &self.presenter_pubkey])
    }

    fn outer_journal_bytes(&self) -> Vec<u8> {
        let journal = OuterJournal {
            version: 1,
            inner_image_id: self.expected_inner_image_id,
            gate_context_id: self.context_id(),
            accepted_context_nullifier: self.nullifier,
            accepted_presenter_id: self.presenter_id(),
            accepted_threshold: self.threshold,
        };
        borsh::to_vec(&journal).expect("encode outer journal")
    }
}

#[test]
fn register_presenter_claims_account_and_writes_pubkey() {
    let fixture = Fixture::default_with_nullifier([0x99; 32]);
    let pre_states = vec![
        default_pre_state(fixture.presenter_account_id),
        authorized_pre_state(fixture.gate_account_id, &[0x00; 32]),
    ];
    let instruction = Instruction::RegisterPresenter {
        presenter_pubkey: fixture.presenter_pubkey,
    };

    let output = run_program(BALANCE_ATTESTATION_PROGRAM_ID, &pre_states, &instruction);
    assert_eq!(output.post_states.len(), 2);
    assert_eq!(
        output.post_states[0].required_claim(),
        Some(Claim::Authorized)
    );
    assert_eq!(
        &output.post_states[0].account().data.as_ref()[..32],
        &fixture.presenter_pubkey
    );
}

#[test]
fn init_then_admit_records_nullifier() {
    let fixture = Fixture::default_with_nullifier([0xAA; 32]);

    // 1. InitGate against an uninitialized gate account + an authorized admin.
    let init_pre = vec![
        default_pre_state(fixture.gate_account_id),
        registered_presenter_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ];
    let init_instruction = Instruction::InitGate {
        chain_id: fixture.chain_id,
        verifier_id: fixture.verifier_id,
        gate_id: fixture.gate_id,
        threshold: fixture.threshold,
        expected_inner_image_id: fixture.expected_inner_image_id,
    };
    let init_output = run_program(BALANCE_ATTESTATION_PROGRAM_ID, &init_pre, &init_instruction);
    assert_eq!(init_output.post_states.len(), 2);
    let mut initialized_gate = init_output.post_states[0].account().clone();
    let stored: GateState = BorshDeserialize::try_from_slice(initialized_gate.data.as_ref())
        .expect("decode initialized gate state");
    assert_eq!(stored.magic, GATE_STATE_MAGIC);
    assert_eq!(stored.version, 1);
    assert_eq!(stored.threshold, fixture.threshold);
    assert!(stored.admitted_nullifiers.is_empty());
    // The guest leaves program_owner = DEFAULT_PROGRAM_ID and emits
    // Claim::Authorized so claim resolution can flip ownership. Validate the
    // claim was attached on init, then simulate the claim-resolution step
    // here so the next call sees the gate as owned by the program.
    assert_eq!(
        init_output.post_states[0].required_claim(),
        Some(Claim::Authorized)
    );
    initialized_gate.program_owner = BALANCE_ATTESTATION_PROGRAM_ID;

    // 2. Admit consumes the initialized gate + an authorized presenter,
    //    appends the nullifier.
    let admit_pre = vec![
        AccountWithMetadata {
            account: initialized_gate,
            is_authorized: false,
            account_id: fixture.gate_account_id,
        },
        registered_presenter_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ];
    let admit_instruction = Instruction::Admit {
        outer_journal: fixture.outer_journal_bytes(),
    };
    let admit_output = run_program(
        BALANCE_ATTESTATION_PROGRAM_ID,
        &admit_pre,
        &admit_instruction,
    );
    let admitted_gate = admit_output.post_states[0].account().clone();
    let stored_after: GateState = BorshDeserialize::try_from_slice(admitted_gate.data.as_ref())
        .expect("decode admitted state");
    assert_eq!(stored_after.admitted_nullifiers, vec![fixture.nullifier]);
}

#[test]
fn admit_replay_panics_with_ba206() {
    let fixture = Fixture::default_with_nullifier([0xBB; 32]);
    let already_admitted = make_initialized_gate_state(&fixture, vec![fixture.nullifier]);

    let pre_states = vec![
        AccountWithMetadata {
            account: already_admitted,
            is_authorized: false,
            account_id: fixture.gate_account_id,
        },
        registered_presenter_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ];
    let instruction = Instruction::Admit {
        outer_journal: fixture.outer_journal_bytes(),
    };

    let err = run_program_err(BALANCE_ATTESTATION_PROGRAM_ID, &pre_states, &instruction);
    assert!(
        err.contains("BA206"),
        "expected BA206 DuplicateNullifier, got: {err}"
    );
}

#[test]
fn admit_with_wrong_presenter_panics_with_ba203() {
    let fixture = Fixture::default_with_nullifier([0xCC; 32]);
    let initialized = make_initialized_gate_state(&fixture, vec![]);
    let wrong_pubkey = [0xEE; 32];

    let pre_states = vec![
        AccountWithMetadata {
            account: initialized,
            is_authorized: false,
            account_id: fixture.gate_account_id,
        },
        registered_presenter_pre_state(fixture.presenter_account_id, &wrong_pubkey),
    ];
    let instruction = Instruction::Admit {
        outer_journal: fixture.outer_journal_bytes(),
    };

    let err = run_program_err(BALANCE_ATTESTATION_PROGRAM_ID, &pre_states, &instruction);
    assert!(
        err.contains("BA203"),
        "expected BA203 PresenterMismatch, got: {err}"
    );
}

#[test]
fn admit_with_wrong_inner_image_id_panics_with_ba102() {
    let fixture = Fixture::default_with_nullifier([0xDD; 32]);
    let initialized = make_initialized_gate_state(&fixture, vec![]);

    let mut journal = OuterJournal {
        version: 1,
        inner_image_id: [0xFF; 32], // wrong
        gate_context_id: fixture.context_id(),
        accepted_context_nullifier: fixture.nullifier,
        accepted_presenter_id: fixture.presenter_id(),
        accepted_threshold: fixture.threshold,
    };
    // Recompute context id with the wrong inner image id so the inner-image
    // mismatch error fires before the context-mismatch error.
    journal.gate_context_id = hash_segments(&[
        CONTEXT_DOMAIN,
        &fixture.chain_id,
        &journal.inner_image_id,
        &fixture.verifier_id,
        &fixture.gate_id,
        &fixture.threshold.to_le_bytes(),
    ]);

    let pre_states = vec![
        AccountWithMetadata {
            account: initialized,
            is_authorized: false,
            account_id: fixture.gate_account_id,
        },
        registered_presenter_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ];
    let instruction = Instruction::Admit {
        outer_journal: borsh::to_vec(&journal).unwrap(),
    };

    let err = run_program_err(BALANCE_ATTESTATION_PROGRAM_ID, &pre_states, &instruction);
    assert!(
        err.contains("BA102"),
        "expected BA102 InvalidImageId, got: {err}"
    );
}

#[test]
fn program_id_is_pinned() {
    // Alarm if the guest ELF silently changes (image-id determinism).
    assert_eq!(
        BALANCE_ATTESTATION_PROGRAM_ID,
        [
            1814270443, 2715932601, 2328945781, 1735716110, 1384216943, 615796249, 572526872,
            2343830086
        ],
        "BALANCE_ATTESTATION_PROGRAM_ID drift detected — recompute and update the pin if intended"
    );
}

// --- helpers ---

fn run_program(
    program_id: ProgramId,
    pre_states: &[AccountWithMetadata],
    instruction: &Instruction,
) -> ProgramOutput {
    let env = build_env(program_id, pre_states, instruction);
    let session = default_executor()
        .execute(env, BALANCE_ATTESTATION_PROGRAM_ELF)
        .expect("guest executed");
    session.journal.decode().expect("decode program output")
}

fn run_program_err(
    program_id: ProgramId,
    pre_states: &[AccountWithMetadata],
    instruction: &Instruction,
) -> String {
    let env = build_env(program_id, pre_states, instruction);
    match default_executor().execute(env, BALANCE_ATTESTATION_PROGRAM_ELF) {
        Ok(_) => panic!("guest unexpectedly succeeded"),
        Err(e) => e.to_string(),
    }
}

fn build_env<'a>(
    program_id: ProgramId,
    pre_states: &'a [AccountWithMetadata],
    instruction: &'a Instruction,
) -> ExecutorEnv<'a> {
    let mut builder = ExecutorEnv::builder();
    let caller_program_id: Option<ProgramId> = None;
    let pre_states_owned = pre_states.to_vec();
    let instruction_data: Vec<u32> = to_vec(instruction).expect("serialize instruction");
    builder
        .write(&program_id)
        .and_then(|b| b.write(&caller_program_id))
        .and_then(|b| b.write(&pre_states_owned))
        .and_then(|b| b.write(&instruction_data))
        .expect("write inputs");
    builder.build().expect("build env")
}

fn default_pre_state(account_id: AccountId) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account::default(),
        is_authorized: true,
        account_id,
    }
}

fn authorized_pre_state(account_id: AccountId, pubkey: &[u8; 32]) -> AccountWithMetadata {
    let data = pubkey.to_vec().try_into().expect("pubkey fits in data");
    AccountWithMetadata {
        account: Account {
            data,
            ..Account::default()
        },
        is_authorized: true,
        account_id,
    }
}

fn registered_presenter_pre_state(account_id: AccountId, pubkey: &[u8; 32]) -> AccountWithMetadata {
    let mut state = authorized_pre_state(account_id, pubkey);
    state.account.program_owner = BALANCE_ATTESTATION_PROGRAM_ID;
    state
}

fn make_initialized_gate_state(fixture: &Fixture, admitted_nullifiers: Vec<[u8; 32]>) -> Account {
    let state = GateState {
        magic: GATE_STATE_MAGIC,
        version: 1,
        chain_id: fixture.chain_id,
        verifier_id: fixture.verifier_id,
        gate_id: fixture.gate_id,
        threshold: fixture.threshold,
        expected_inner_image_id: fixture.expected_inner_image_id,
        admitted_nullifiers,
    };
    let bytes = borsh::to_vec(&state).expect("encode gate state");
    let data = bytes.try_into().expect("gate state fits in data");
    Account {
        program_owner: BALANCE_ATTESTATION_PROGRAM_ID,
        data,
        ..Account::default()
    }
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for seg in segments {
        hasher.update((seg.len() as u64).to_le_bytes());
        hasher.update(seg);
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}
