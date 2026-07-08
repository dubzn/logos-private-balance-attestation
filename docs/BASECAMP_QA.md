# Basecamp QA Checklist

Last updated: 2026-07-01.

This checklist is for the local Basecamp MVP. It does not replace the terminal
E2E scripts; it confirms that the `ui_qml` module packages, loads, and drives
the same CLI/script flow from the UI.

## Automated Package Check

From the repository root:

```sh
scripts/check-basecamp-package.sh
```

This runs `nix build .#install` under `apps/basecamp/` and verifies:

- `result/plugins/balance_attestation/` exists
- `manifest.json` and `metadata.json` describe a `ui_qml` module
- `src/qml/BalanceAttestation.qml` is present
- `balance_attestation_plugin.*` exists
- `balance_attestation_replica_factory.*` exists
- `delivery_module` is declared as a dependency

The same check is available through the public clean-room command:

```sh
ALLOW_DIRTY=1 scripts/check-public-clean-room.sh --with-basecamp
```

Or through the root demo entrypoint:

```sh
./demo.sh --clean-room --real-prover --with-basecamp
```

## Manual Install

The shortest local launch path is:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

This builds and inspects the Basecamp package, copies it into the local
Basecamp user plugin directory, and launches the Nix-built Basecamp dev app in
the foreground. Keep that terminal open while using the UI.

The manual equivalent is below.

Build:

```sh
cd apps/basecamp
nix build .#install
```

Install into a local Basecamp user directory:

```sh
export BASECAMP_USER_DIR=/Users/dub/Desktop/logos/basecamp-balance-attestation-user

rm -rf "$BASECAMP_USER_DIR/plugins/balance_attestation"
mkdir -p "$BASECAMP_USER_DIR/plugins"
cp -R result/plugins/balance_attestation "$BASECAMP_USER_DIR/plugins/"
chmod -R u+w "$BASECAMP_USER_DIR/plugins/balance_attestation"
```

Launch Basecamp from the local Nix-built checkout:

```sh
BALANCE_ATTEST_REPO=/Users/dub/Desktop/logos/logos-private-balance-attestation \
LOGOS_LEZ_REPO=/Users/dub/Desktop/logos/logos-execution-zone \
NSSA_WALLET_HOME_DIR=/Users/dub/Desktop/logos/logos-execution-zone/.wallet-local \
RISC0_DEV_MODE=0 \
  /Users/dub/Desktop/logos/logos-basecamp/run-dev.sh --user-dir "$BASECAMP_USER_DIR"
```

Load `balance_attestation` from `Modules -> UI Modules`.

## Preconditions

Before pressing UI actions:

1. Start the local LEZ sequencer from the same `LOGOS_LEZ_REPO`.
2. Run:

   ```sh
   scripts/check-wallet-preflight.sh
   ```

3. Make sure the private account is initialized and funded:

   ```sh
   PRIVATE_ACCOUNT=Private/<private-account-id> \
   THRESHOLD=1 \
   RISC0_DEV_MODE=0 \
     scripts/prepare-local-private-account.sh
   ```

## Manual UI Flow

Use the app fields:

| Field | Expected value |
| --- | --- |
| Repository | `/Users/dub/Desktop/logos/logos-private-balance-attestation` |
| LEZ checkout | `/Users/dub/Desktop/logos/logos-execution-zone` |
| Wallet home | `/Users/dub/Desktop/logos/logos-execution-zone/.wallet-local` |
| Private account | `Private/<private-account-id>` |
| Threshold | `1` |
| Real proving | enabled for final recording |

Then run:

1. **Preflight**
   - Expected: status ends with wallet/sequencer preflight ok.
   - If it fails, fix the terminal preflight before continuing.

2. **Generate proof**
   - Expected: Proof card becomes `ok`.
   - Expected: `Proof JSON` tab contains `status: ok`.
   - Evidence: copy the proof run dir shown in the Proof card.
   - If the status says the private account was not found, keep the same wallet
     home and copy one of the listed `Private/<id>` values into the Private
     account field, or switch Wallet home to the directory that owns the
     selected account.

3. **Verify envelope**
   - Expected: Verify card becomes `ok`.
   - Expected: `Verify JSON` shows `context_nullifier`, `presenter_id`, and
     `threshold`.

4. **Gate admit**
   - Expected: Gate card becomes `ok`.
   - Expected: `Gate JSON` shows nullifier count `1` and duplicate admission
     behavior as `not-applied`.

5. **Delivery send**
   - Press **Create node** in the Delivery toolbar.
   - Expected: Delivery card shows a peer id or node status.
   - Press **Subscribe**.
   - Expected: Delivery log records the subscribed topic.
   - Press **Send proof**.
   - Expected: Delivery log records a request id and `Delivery Msg` contains a
     V1 proof message with an embedded public envelope.

6. **Delivery receive / verify**
   - In a second Basecamp instance using a different user directory, use the
     same Delivery topic and gate/context fields.
   - Press **Create node**, then **Subscribe**.
   - Expected: `Delivery Msg` receives the proof message.
   - Press **Verify received**.
   - Expected: `Delivery Verify` reports `status: ok`.

7. **Clear**
   - Expected: cards return to idle/empty public output state.

## What To Capture For The Video

- The app visible under `Modules -> UI Modules` as `balance_attestation`.
- The `Real proving` toggle enabled.
- The Preflight action finishing successfully.
- Proof generation output with `RISC0_DEV_MODE=0`.
- Verify output with `status: ok`.
- Gate output with a persisted context nullifier.
- Delivery node creation, subscription, proof send, proof receive, and
  `Verify received` output with `status: ok`.
- A short terminal view of the generated report directory, without opening or
  publishing `witness.json`.

## Known Limits To Say Out Loud

- The Basecamp UI is an MVP local operator surface.
- It intentionally delegates to the same CLI/scripts as the terminal demo.
- It does not parse or display `witness.json`.
- The current gate button uses the Workable host-preverified gate path.
- The PPE-native gate candidate is currently terminal-only.
- Delivery uses the real Basecamp `delivery_module`; the CLI `./demo.sh
  --messaging` path remains a deterministic local JSON fallback for CI and
  clean-room checks.
- Public LEZ in-guest verification of an externally submitted RISC Zero receipt
  remains pending Logos/evaluator guidance.
