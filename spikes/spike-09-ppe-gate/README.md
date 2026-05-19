# Spike 09: PPE-Native Balance Gate

This spike tests the Logos-native private execution path as the possible
LP-0005 on-chain verifier model.

## Question

```text
Can a LEZ privacy-preserving transaction prove a private account has
balance >= threshold and update a public gate/nullifier ledger in one
sequencer-verified transaction?
```

This is different from the current Workable / host-preverified path:

- Workable path: host verifies the portable proof envelope, then submits a
  public LEZ gate-ledger transaction.
- PPE-native path: the LEZ private execution circuit itself proves the private
  balance check while the program writes public gate state.

If Logos evaluators accept PPE-native verification as the expected LP-0005
on-chain path, this is the strongest local direction we have found so far.

## Shape

The program receives three accounts:

1. private holder account
2. public gate state account
3. public presenter account

The guest checks:

```text
private_holder.is_authorized == true
private_holder.balance >= threshold
presenter.is_authorized == true
gate is default or already owned by this program
context_nullifier has not been admitted before
```

The public gate account stores a borsh state:

```text
magic = "BAP1"
version = 1
chain_id
verifier_id
gate_id
threshold
admissions = [(context_nullifier, presenter_account_id)]
```

The nullifier is derived inside the guest:

```text
SHA256(domain || private_holder_account_id || context_id)
```

The private holder account id is not written to public state. Reusing the same
private holder against the same gate context produces the same nullifier, so a
second admission is rejected.

## What This Proves

- The on-chain gate action can be driven by a sequencer-verified private LEZ
  transaction.
- The balance threshold is checked inside the private execution proof, not by a
  host-only precheck.
- A public gate ledger can record a nullifier and presenter account without
  revealing the holder's balance.

## What This Does Not Prove

- It does not verify the same standalone RISC Zero proof envelope used by the
  off-chain path.
- It does not transmit a reusable credential over Messaging.
- It still needs evaluator confirmation that "LEZ private execution verifies
  the balance condition" satisfies LP-0005's on-chain verifier requirement.

## Commands

Build:

```sh
LOGOS_LEZ_REPO=/path/to/logos-execution-zone \
  scripts/spike-09-build-ppe-gate.sh
```

Run against existing accounts:

```sh
PRIVATE_ACCOUNT=Private/<private-id> \
GATE_ACCOUNT=Public/<gate-id> \
PRESENTER_ACCOUNT=Public/<presenter-id> \
THRESHOLD=1 \
LOGOS_LEZ_REPO=/path/to/logos-execution-zone \
  scripts/spike-09-run-ppe-gate.sh
```

Automated local fixture:

```sh
LOGOS_LEZ_REPO=/path/to/logos-execution-zone \
NSSA_WALLET_HOME_DIR=/path/to/logos-execution-zone/.wallet-local \
RISC0_DEV_MODE=0 \
  scripts/spike-09-demo-ppe-gate.sh
```

The automated fixture creates fresh local accounts, funds the private holder,
admits once, verifies the nullifier is recorded, syncs private state, and then
checks that a duplicate admission does not append a second nullifier.

`RISC0_DEV_MODE` must match the mode used by the running sequencer. The latest
validated run used `RISC0_DEV_MODE=0`; a mismatch can make private funding fail
or leave the private holder uninitialized.
