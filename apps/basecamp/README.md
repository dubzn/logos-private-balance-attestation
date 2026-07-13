# Basecamp MVP

This is a backend-backed `ui_qml` module for the LP-0005 local flow.

The QML surface calls a small C++ backend, and the backend delegates to the same
CLI/scripts used by the reproducible terminal demo:

- `scripts/check-wallet-preflight.sh`
- `scripts/prepare-local-private-account.sh`
- `scripts/demo-local-sequencer-e2e.sh`
- `cargo run -p attestation-cli -- verify`
- `scripts/demo-local-gate-e2e.sh`

The app intentionally does not parse or display `witness.json`. It only shows
public run summaries and verifier/gate outputs.

## Current Actions

- configure attestation repo, LEZ checkout, wallet home, private account, and
  threshold
- edit context fields used by the proof harness
- run wallet/sequencer preflight
- prepare a local private account from the terminal when the account is not yet
  initialized/funded
- generate a proof from real wallet private state and `getProofForCommitment`
- verify the public envelope locally
- submit the current Workable gate admit flow
- create/start a Logos Delivery node through `delivery_module`
- subscribe to the LP-0005 proof-message topic
- send the generated proof envelope as a Delivery proof message
- receive and verify a Delivery proof message locally

## Build Check

From this directory:

```sh
nix build .#install
```

From the repository root, the automated package inspection is:

```sh
scripts/check-basecamp-package.sh
```

The repository-level build/install/launch helper is:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

It installs both the UI plugin and its Delivery dependency into the selected
Basecamp user directory:

```text
<user-dir>/plugins/balance_attestation/
<user-dir>/modules/delivery_module/
```

The build output is a Basecamp install tree:

```text
result/plugins/balance_attestation/
```

For a local Basecamp dev instance, install it under the Basecamp user
`plugins/` directory, not `modules/`:

```sh
export BASECAMP_USER_DIR=/Users/dub/Desktop/logos/basecamp-balance-attestation-user

rm -rf "$BASECAMP_USER_DIR/plugins/balance_attestation"
mkdir -p "$BASECAMP_USER_DIR/plugins"
cp -R result/plugins/balance_attestation "$BASECAMP_USER_DIR/plugins/"
chmod -R u+w "$BASECAMP_USER_DIR/plugins/balance_attestation"
```

Then restart Basecamp with the same user dir. The app should appear in
`Modules -> UI Modules` as `balance_attestation`.

Install the Delivery dependency into the same user dir when using the manual
copy path:

```sh
DELIVERY_INSTALL="$(nix build --print-out-paths .#delivery-install)"
rm -rf "$BASECAMP_USER_DIR/modules/delivery_module"
mkdir -p "$BASECAMP_USER_DIR/modules"
cp -R "$DELIVERY_INSTALL/modules/delivery_module" "$BASECAMP_USER_DIR/modules/"
chmod -R u+w "$BASECAMP_USER_DIR/modules/delivery_module"
```

The Delivery commit is pinned because it uses `logos-cpp-sdk 0.2.0`, including
the provider-event thread marshalling required by the two-instance workflow.
The app flake also applies the narrow patch under
`nix/logos-cpp-sdk-patched/`: the current cdylib code generator serializes
`bstr` event arguments as empty tagged bytes. Without that patch, Delivery
propagates each chunk but Basecamp receives `messageReceived(..., bytes=0)`.
The wrapper is pinned to an official SDK commit and can be removed once the
same fix lands upstream. Upstream tracking:
[`logos-cpp-sdk#99`](https://github.com/logos-co/logos-cpp-sdk/issues/99).

The install tree must contain both backend libraries:

```text
balance_attestation_plugin.dylib
balance_attestation_replica_factory.dylib
```

The generated metadata must also declare:

```json
"dependencies": ["delivery_module"]
```

A direct `ui-host` smoke test should reach `READY`; otherwise Basecamp will
show the plugin but time out while loading it.

If Nix/Qt is unavailable, the repository-level smoke check still validates the
QML/C++ files for public hygiene and shell flow compatibility:

```sh
cd ../..
scripts/check-public-clean-room.sh --with-lez
```

For the full local QA checklist, see
[`docs/BASECAMP_QA.md`](../../docs/BASECAMP_QA.md).

## Runtime Notes

The backend defaults to the side-by-side checkout layout:

```text
workdir/
  logos-private-balance-attestation/
  logos-execution-zone/
```

Set `BALANCE_ATTEST_REPO` and `LOGOS_LEZ_REPO` if Basecamp launches from a
different working directory.

Before pressing **Generate Proof**, make sure the selected private account is
ready:

```sh
cd /path/to/logos-private-balance-attestation

PRIVATE_ACCOUNT=Private/<id> \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  scripts/prepare-local-private-account.sh
```

Fresh private accounts created with `wallet account new private` are only local
wallet keys at first. They must be initialized and funded before the app can
build a membership proof from `getProofForCommitment`.

If **Generate Proof** reports that the private account was not found, the app is
protecting you from a common local setup mismatch: the `Private/<id>` in the UI
does not belong to the selected wallet home. Either switch **Wallet home** to
the directory that owns that account, or copy one of the private accounts listed
in the status panel into the **Private account** field.

The gate action uses the documented Workable path: host-side proof verification
followed by a deployable LEZ gate-ledger/nullifier transaction. The deployed LEZ
program does not yet verify the RISC Zero receipt inside public LEZ execution.

## Logos Delivery Flow

The Delivery panel is the real Logos Messaging/Delivery adapter for this MVP.
It uses the Basecamp-provided `delivery_module`, not the local JSON transport.

Typical sender flow:

1. Run **Preflight**.
2. Run **Generate proof**.
3. Run **Verify envelope**.
4. In **Logos Delivery**, press **Create node**.
5. Press **Subscribe**.
6. Press **Send proof**.

Typical receiver flow in another Basecamp instance/user dir:

1. Use the same Delivery topic and gate/context fields.
2. Press **Create node**.
3. Press **Subscribe**.
4. Wait for **Delivery Msg** to show the received proof message.
5. Press **Verify received**.

The receiver log should show each non-empty chunk, a successful SHA-256
reassembly, and then `status: ok` after verification.

This exact path was validated across two Basecamp instances on 2026-07-13. A
1,323,577-byte real-prover message arrived in 17 out-of-order chunks,
reassembled to SHA-256
`f042487460a8485f144d0735577d4163ee63cde38860f4a270da07462b63a075`,
and passed `message-verify` with `status: ok`.

`Verify received` delegates to:

```sh
cargo run -p attestation-cli -- message-verify \
  --message <delivery-run>/proof-message.json \
  --gate <delivery-run>/gate.json
```

The received message contains only the public proof envelope. It never contains
`witness.json`.
