# Basecamp MVP

This is a backend-backed `ui_qml` module for the LP-0005 local flow.

The QML surface calls a small C++ backend, and the backend delegates to the same
CLI/scripts used by the reproducible terminal demo:

- `scripts/check-wallet-preflight.sh`
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
- generate a proof from real wallet private state and `getProofForCommitment`
- verify the public envelope locally
- submit the current Workable gate admit flow

## Build Check

From this directory:

```sh
nix build .#install
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

The install tree must contain both backend libraries:

```text
balance_attestation_plugin.dylib
balance_attestation_replica_factory.dylib
```

A direct `ui-host` smoke test should reach `READY`; otherwise Basecamp will
show the plugin but time out while loading it.

If Nix/Qt is unavailable, the repository-level smoke check still validates the
QML/C++ files for public hygiene and shell flow compatibility:

```sh
cd ../..
scripts/check-public-clean-room.sh --with-lez
```

## Runtime Notes

The backend defaults to the side-by-side checkout layout:

```text
workdir/
  logos-private-balance-attestation/
  logos-execution-zone/
```

Set `BALANCE_ATTEST_REPO` and `LOGOS_LEZ_REPO` if Basecamp launches from a
different working directory.

The gate action uses the documented Workable path: host-side proof verification
followed by a deployable LEZ gate-ledger/nullifier transaction. The deployed LEZ
program does not yet verify the RISC Zero receipt inside public LEZ execution.
