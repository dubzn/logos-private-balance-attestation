# Testnet Deployment Evidence

Last updated: 2026-06-01.

This document records public LEZ testnet evidence for the two LP-0005 on-chain
candidate paths currently implemented in this repository.

Endpoint:

```text
https://testnet.lez.logos.co
```

Important version note:

```text
Public testnet program IDs match logos-execution-zone tag v0.1.2.
```

This was verified by comparing `getProgramIds` from the public endpoint against
the generated `nssa` program IDs from `logos-execution-zone` tags. `upstream/main`
did not match the public testnet at the time of this run.

## Environment

| Field | Value |
| --- | --- |
| Date | 2026-06-01 |
| LEZ tag | `v0.1.2` |
| LEZ commit | `cf3639d8252040d13b3d4e933feb19b42c76e14a` |
| Testnet endpoint | `https://testnet.lez.logos.co` |
| RISC Zero mode | `RISC0_DEV_MODE=0` for the PPE-native run |
| Wallet material | Throwaway local testnet wallet; private material omitted |

The throwaway wallet home and raw logs are intentionally not committed.

## Program ID Discovery

Query the public endpoint:

```sh
curl -s -X POST https://testnet.lez.logos.co \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getProgramIds","params":[]}' | jq .
```

The returned built-in program IDs matched `logos-execution-zone` `v0.1.2`.
Using `upstream/main` failed wallet health with a program-id mismatch, so all
testnet deploy/admit evidence below was run against `v0.1.2`.

## Path A: Workable Host-Preverified Gate

This is the public LEZ gate-ledger path:

```text
host verifies public proof envelope
  -> host submits LEZ gate transaction
  -> LEZ program records/deduplicates context nullifier
```

This path is useful and deployable, but it is not an in-program receipt
verifier. The host-side `attestation-verifier` precheck is the cryptographic
trust seat.

### Public IDs

| Field | Value |
| --- | --- |
| Program ID, u32 words | `[132217456, 1022839988, 453626531, 1132126476, 3831540952, 652159567, 3230477833, 332845675]` |
| Program ID, account encoding | `8a5LYc1Hb8oXv8deTzhR4jz4Eq9YXw6AL5y8wapDe1Ni` |
| Gate account | `Public/BWZScqFEwVyTqVXQBya6PxFTU1Lmr92MyJECPiQgEriG` |
| Presenter account | `Public/5WZ4MtzMPbK2f2xtgpoNkuodMfumy8YDapah6A6qhAdC` |
| Context ID | `cede10e4710312e5974d324d107fca56799a8ea9423c3fcf3f9d73d21db11011` |
| Context nullifier | `18d81747e25c7e30f5e4ca6ebe9b49f15a8b5e88173112ddcb0014a1d1233151` |

### Transaction Hashes

| Operation | Transaction hash | Result |
| --- | --- | --- |
| Register presenter | `f9d9f157f192f4675f12c8fd4ae8d44d060dfa280f937b48704d6625bcabce5f` | Applied |
| Init gate | `a56e70b9110d2aa710c7d943e8a69829101e0f6e224a9debbdcab131b9c63433` | Applied |
| Admit | `ba1feb25a14aeff7a566f66ad647a4a21a337fca59f2dfead2d115d0bc4e6fdd` | Applied |
| Duplicate admit | `c7c8d42d8dd42c2f04bf2b31a9dde4411ada8e5105886aa011da24eab1597dd8` | Submitted; state did not change |

`wallet deploy-program` on `v0.1.2` produced no transaction hash in stdout. The
program ID above is the deployed program image ID observed by the runner and as
the `program_owner` on the modified gate/presenter accounts.

### Commands

Build against the testnet-matching LEZ checkout:

```sh
export LOGOS_LEZ_REPO=/path/to/logos-execution-zone-v0.1.2
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-testnet-lp0005"
export RISC0_DEV_MODE=0

cargo build -p attestation-cli
cargo build --release --manifest-path lez-verifier/program/Cargo.toml
cargo build --release --manifest-path spikes/spike-08-program-chaining/lez/runner/Cargo.toml
```

Deploy and run:

```sh
wallet deploy-program \
  lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin

target/debug/balance-attest gate-register-presenter \
  --presenter-account Public/5WZ4MtzMPbK2f2xtgpoNkuodMfumy8YDapah6A6qhAdC \
  --admin-account Public/2Y8rTL738CjXCEBPMvbjJvxuH5Um7oKswfcVJbVUcaVF \
  --presenter-pubkey-hex 9ac20335eb38768d2052be1dbbc3c8f6178407458e51e6b4ad22f1d91758895b \
  --execute \
  --wallet-home "$NSSA_WALLET_HOME_DIR" \
  --program-bin lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin \
  --runner-bin spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run \
  --skip-build

target/debug/balance-attest gate-init \
  --gate .demo-runs/local-full/20260519T000012Z/proof/gate.json \
  --gate-account Public/BWZScqFEwVyTqVXQBya6PxFTU1Lmr92MyJECPiQgEriG \
  --admin-account Public/5tBejxNbtnEPNUYPnnNKsNtQt1zM7vvAm1a12qtJc15x \
  --execute \
  --wallet-home "$NSSA_WALLET_HOME_DIR" \
  --program-bin lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin \
  --runner-bin spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run \
  --skip-build

target/debug/balance-attest gate-admit \
  --envelope .demo-runs/local-full/20260519T000012Z/proof/envelope.json \
  --gate .demo-runs/local-full/20260519T000012Z/proof/gate.json \
  --gate-account Public/BWZScqFEwVyTqVXQBya6PxFTU1Lmr92MyJECPiQgEriG \
  --presenter-account Public/5WZ4MtzMPbK2f2xtgpoNkuodMfumy8YDapah6A6qhAdC \
  --execute \
  --wallet-home "$NSSA_WALLET_HOME_DIR" \
  --program-bin lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin \
  --runner-bin spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run \
  --skip-build
```

### Readback Evidence

Presenter account:

```json
{"balance":0,"program_owner":"8a5LYc1Hb8oXv8deTzhR4jz4Eq9YXw6AL5y8wapDe1Ni","data":"9ac20335eb38768d2052be1dbbc3c8f6178407458e51e6b4ad22f1d91758895b","nonce":1}
```

Gate account after admit:

```json
{"balance":0,"program_owner":"8a5LYc1Hb8oXv8deTzhR4jz4Eq9YXw6AL5y8wapDe1Ni","data":"42415431010010101010101010101010101010101010101010101010101010101010101010102020202020202020202020202020202020202020202020202020202020202020303030303030303030303030303030303030303030303030303030303030303001000000000000000000000000000000298509ccee1753d4f037a5b1ff6576c42c21073651743d73dc3fb627a0f93c050100000018d81747e25c7e30f5e4ca6ebe9b49f15a8b5e88173112ddcb0014a1d1233151","nonce":2}
```

The `data` starts with `42415431` (`BAT1`) and contains the expected context
nullifier exactly once. After the duplicate admit transaction settled, the gate
account was unchanged and still had `nonce: 2`.

## Path B: PPE-Native Gate Candidate

This is the stronger Logos-native private execution candidate:

```text
private holder account
  -> LEZ privacy-preserving execution checks balance >= threshold
  -> public BAP1 gate/nullifier state is written
```

This path does prove the private balance condition inside LEZ private execution.
It does not verify the same portable proof envelope used by the off-chain path,
so evaluator confirmation is still required before calling it the final LP-0005
on-chain verifier model.

### Public IDs

| Field | Value |
| --- | --- |
| Program ID, u32 words | `[411233179, 1364156298, 540691977, 2827788750, 2267394079, 1887386629, 231760863, 2207022502]` |
| Gate account | `Public/EW1DxWaLqgoHhVSh5mRiSzqYUjzna93YyzNTkMqiKa6W` |
| Presenter account | `Public/6kbhz5JDJyypyN4B73Rs6muzJ8H9ABKQ8yZUsxCb7GRE` |
| Context nullifier | `4ac5eb012f03c9efc572ad3e4c2f12517803d8e0071e799e4ef98db32c4c81d9` |

The private holder account id and wallet private material are intentionally
omitted.

### Transaction Hashes

| Operation | Transaction hash | Result |
| --- | --- | --- |
| Funding account init | `2bf2e44813a0abe3d5ab3cc094c30e26274c8c5f1e12c07204d645411008802f` | Applied |
| Pinata claim | `42144f43d9c1a923edad3dd72d223d09017192027623a1a5fca839ef4b9d5bd2` | Applied after wallet wait timeout; observed balance `150` |
| Public-to-private funding transfer | `ddb9a3dac9da7bbf10dae63e225d0a5a51a10bd72558dffcab77eebfe6afea22` | Applied; private balance `42` observed |
| Positive PPE admit | `2327cc599f8c6662e93e36e9edc353bd6def3c6720f8f2707d1a8eb5b43068c2` | Applied; nullifier recorded |
| Duplicate PPE admit | none | Rejected during proving with `BA206 DuplicateNullifier` |
| Insufficient-balance admit | none | Rejected during proving with `BA201 ThresholdMismatch` |

### Command

```sh
export LOGOS_LEZ_REPO=/path/to/logos-execution-zone-v0.1.2
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-testnet-lp0005"
export RISC0_DEV_MODE=0
export SKIP_BUILD=1
export PROGRAM_BIN="$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/ppe_balance_gate.bin"

scripts/spike-09-demo-ppe-gate.sh
```

The run produced:

```text
Spike 09 PPE-native gate demo passed.
Gate account: Public/EW1DxWaLqgoHhVSh5mRiSzqYUjzna93YyzNTkMqiKa6W
Presenter account: Public/6kbhz5JDJyypyN4B73Rs6muzJ8H9ABKQ8yZUsxCb7GRE
Context nullifier: 4ac5eb012f03c9efc572ad3e4c2f12517803d8e0071e799e4ef98db32c4c81d9
```

### Timings

| Phase | Duration |
| --- | ---: |
| Wallet/sequencer health | 00:00:01 |
| Deploy PPE program | 00:00:32 |
| Create accounts | 00:00:00 |
| Fund private holder | 00:09:00 |
| Positive admit | 00:03:14 |
| Duplicate admit | 00:00:24 |
| Insufficient-balance rejection | 00:00:03 |
| Total | 00:13:16 |

## CU / Gas Status

These runs provide public testnet transaction hashes, account readback evidence,
and wall-clock timings. They do not yet provide compute-unit cost because the
wallet/RPC path used here did not expose per-transaction CU data.

Open CU items:

- CU for Workable `register_presenter`.
- CU for Workable `init_gate`.
- CU for Workable `admit`.
- CU or rejection cost for Workable duplicate admit, if exposed.
- CU for PPE-native positive admit.
- CU or prover/rejection cost for PPE-native `BA206` and `BA201` paths, if
  evaluators want those treated as chain operations.

## Current Interpretation

The project now has public testnet evidence for both candidate paths:

- Path A is deployable and persists/deduplicates nullifiers on testnet, but its
  cryptographic proof verification remains host-side.
- Path B proves the balance gate inside LEZ private execution and writes public
  nullifier state on testnet, but it is not the same portable proof envelope
  used by the off-chain verifier.

The remaining LP-0005 decision is still the same narrow evaluator question:
which on-chain model is expected for final acceptance?
