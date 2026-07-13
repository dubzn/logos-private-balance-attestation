# Benchmarks

Last updated: 2026-07-13

This document records measured runs for LP-0005 without turning wall-clock or
RISC Zero cycle numbers into unsupported chain compute-unit claims. It includes
a full local E2E against a local LEZ sequencer and a deterministic guest-cycle
benchmark modeled on upstream LEZ `tools/cycle_bench`. Public testnet
deployment/admission evidence is recorded separately in
[TESTNET_DEPLOYMENT.md](TESTNET_DEPLOYMENT.md).

## Scope And Caveats

- Run source:
  `.demo-runs/local-full/20260710T225915Z/run.json`
- Proof source:
  `.demo-runs/local-full/20260710T225915Z/proof/run.json`
- Gate source:
  `.demo-runs/local-full/20260710T225915Z/gate/run.json`
- `RISC0_DEV_MODE=0`.
- The proof path uses real wallet private state and the real wallet/sequencer
  membership-proof path.
- The gate path is the current Workable model: host-side proof verification via
  `attestation-verifier`, followed by a LEZ gate-ledger transaction that records
  and deduplicates the context nullifier.
- Spike 09 PPE-native gate timing is included below as separate local evidence.
- The E2E and testnet tables are wall-clock timings, not LEZ CU measurements.
- The 2026-07-10 local run used a clean latest LEZ checkout tracking
  `upstream/dev` at `1b4d8fbc`, including the current `lee_core` and
  `lez/wallet` layout.
- The cycle benchmark below follows the metric and execution shape used by
  upstream `tools/cycle_bench`: RISC Zero `SessionInfo::cycles()` over the
  deployable guest ELF.
- Build steps are included where the scripts include them, so warmed-run timings
  may be lower.
- `witness.json` is private and must not be published.

## Full Local E2E

Command shape:

```sh
PRIVATE_ACCOUNT=Private/<id> RISC0_DEV_MODE=0 scripts/demo-local-full-e2e.sh
```

Run result:

| Metric | Value |
| --- | --- |
| Status | `ok` |
| Verify status | `ok` |
| RISC Zero mode | `RISC0_DEV_MODE=0` |
| Gate account | `Public/BysJqfnFeKdCnJc3Np4vTDRrDeXpCrjmo3T8Hp8zaK1s` |
| Presenter account | `Public/AtfSjcnBATpgLvVR6i4wLLBjycJrPw11T213AT8C43uY` |
| Context nullifier | `8abf1b416cbab7eadf30d28cc10164d5486156304fc15942db62432497210b70` |
| Nullifier count in gate data | `1` |
| Duplicate admit | `not-applied` |

Phase timing:

| Phase | Duration | Output |
| --- | ---: | --- |
| Proof | 00:01:49 | `.demo-runs/local-full/20260710T225915Z/proof/report.md` |
| Gate | 00:01:47 | `.demo-runs/local-full/20260710T225915Z/gate/report.md` |
| Total | 00:03:36 | `.demo-runs/local-full/20260710T225915Z/report.md` |

## Proof Benchmark

This phase builds a witness from private wallet state, proves
`balance >= threshold`, and verifies the public envelope locally.

| Step | Duration | Notes |
| --- | ---: | --- |
| Build | 00:00:02 | Workspace/guest build as invoked by the script. |
| Wallet health | 00:00:00 | Local wallet/sequencer health check. |
| Build witness | 00:01:19 | Reads private wallet state and calls the real `getProofForCommitment` path. |
| Prove | 00:00:24 | RISC Zero proof generation with `RISC0_DEV_MODE=0`. |
| Verify | 00:00:03 | Local off-chain verifier over the public envelope. |
| Total | 00:01:48 | Proof script total from `proof/run.json`. |

Proof metadata:

| Metric | Value |
| --- | --- |
| Threshold | `1` |
| Proof index | `4` |
| Proof depth | `3` |
| Public envelope size | `1,323,175` bytes |
| Gate file size | `366` bytes |
| Verify output size | `373` bytes |

The witness-builder summary confirms:

- private state was found for the selected wallet account
- local commitment reconstruction matched wallet commitment data
- membership proof data was present
- local root reconstruction matched the wallet/sequencer root
- private fields were redacted from logs and reports

## Gate Tx Timings

This phase consumes the public proof envelope and gate file produced by the
proof phase. The CLI performs the mandatory host-side proof precheck before
submitting the gate `admit` transaction.

| Step | Duration | Notes |
| --- | ---: | --- |
| Build | 00:00:03 | CLI, deployable LEZ program, and runner build work. |
| Wallet health + deploy | 00:00:16 | Local wallet health check and program deployment path. |
| Create accounts | 00:00:00 | Fresh public accounts for gate, presenter, and admins. |
| Dry run | 00:00:01 | Prepared register/init/admit commands without execution. |
| Register presenter | 00:00:10 | Stores presenter pubkey in the presenter account. |
| Init gate | 00:00:15 | Initializes gate account with expected context and threshold. |
| Admit | 00:00:16 | Host-verifies envelope, submits admit, observes nullifier in gate data. |
| Duplicate admit | 00:00:46 | Replays same admit and confirms duplicate is not applied after settle. |
| Total | 00:01:47 | Gate script total from `gate/run.json`. |

Local transaction hashes captured by the run:

| Operation | Transaction hash |
| --- | --- |
| Register presenter | `3ea6054bec7db598b67cbe51558d6d7c6a801eb77db002f4e12cd8e8a25b7a7c` |
| Init gate | `81bc68e5da0a1071e95ff17143019b1f42c6553a74ff06a2795d4f28beb14e75` |
| Admit | `7943b9ee65484f99f36b7518554b1bb2087645ef4672bcf2f587275acdd2477a` |
| Duplicate admit | `3197af1aaea44bad73ed4e2797d758ac1fa7039a92728eaea9ab952978fe379e` |

The duplicate transaction was submitted, but the final gate state still contains
one copy of the context nullifier.

## Spike 09 PPE-Native Gate Benchmark

Command shape:

```sh
RISC0_DEV_MODE=0 ./demo.sh --ppe-gate --real-prover
```

Latest run source:

```text
.demo-runs/spike-09-ppe-gate/20260519T201934Z/run.json
.demo-runs/spike-09-ppe-gate/20260519T201934Z/report.md
```

Run result:

| Metric | Value |
| --- | --- |
| Status | `ok` |
| RISC Zero mode | `RISC0_DEV_MODE=0` |
| Gate account | `Public/9iXqs5VabukA532ptUo3SePFWFvMutn2ao8GDX66BFL` |
| Presenter account | `Public/3uUab6SSw1N6DqFu6gqU621wyKVgamjF4DtCV9153bNp` |
| Context nullifier | `f99332b92aa713c9ab22f3df420b0bee5ed10e8b67b055ceeb9bb6545ad0e449` |
| Nullifier count in gate data | `1` |
| Positive path | Private holder balance `42` >= threshold `1`. |
| Negative path | Threshold `43` rejected with `BA201 ThresholdMismatch`. |

Phase timing:

| Phase | Duration | Notes |
| --- | ---: | --- |
| Wallet/sequencer health | 00:00:00 | Local wallet and sequencer compatibility check. |
| Build PPE guest | 00:00:00 | Skipped with `SKIP_BUILD=1`; existing `ppe_balance_gate.bin` reused. |
| Deploy PPE program | 00:00:20 | Local `wallet deploy-program` plus settle wait. |
| Create accounts | 00:00:00 | Fresh funding, private holder, gate, and presenter accounts. |
| Fund private holder | 00:02:21 | Auth-transfer init, Pinata funding, private transfer, private sync. |
| Positive admit | 00:01:49 | PPE transaction proving private `balance >= threshold` and writing `BAP1` gate state. |
| Duplicate admit | 00:00:23 | Replay of same private holder/context rejected or timed out; final nullifier count stayed `1`. |
| Insufficient-balance rejection | 00:00:02 | Threshold above holder balance rejected with `BA201`. |
| Total | 00:04:56 | End-to-end Spike 09 local run. |

The generated report intentionally omits the private holder account id and
wallet private material.

## Public Testnet Wall-Clock Evidence

Public LEZ testnet evidence was collected on 2026-06-01 against
`logos-execution-zone` `v0.1.2`, which matched the public endpoint's built-in
program IDs at the time of the run.

Full evidence, public accounts, transaction hashes, and readback are in
[TESTNET_DEPLOYMENT.md](TESTNET_DEPLOYMENT.md).

### Workable Host-Preverified Gate

| Operation | Transaction hash | Result |
| --- | --- | --- |
| Register presenter | `f9d9f157f192f4675f12c8fd4ae8d44d060dfa280f937b48704d6625bcabce5f` | Applied |
| Init gate | `a56e70b9110d2aa710c7d943e8a69829101e0f6e224a9debbdcab131b9c63433` | Applied |
| Admit | `ba1feb25a14aeff7a566f66ad647a4a21a337fca59f2dfead2d115d0bc4e6fdd` | Applied |
| Duplicate admit | `c7c8d42d8dd42c2f04bf2b31a9dde4411ada8e5105886aa011da24eab1597dd8` | Submitted; gate state unchanged |

The gate account readback showed the expected `BAT1` state and exactly one copy
of the context nullifier after duplicate admit settlement.

### PPE-Native Gate Candidate

| Phase | Duration | Result |
| --- | ---: | --- |
| Wallet/sequencer health | 00:00:01 | Passed |
| Deploy PPE program | 00:00:32 | Applied |
| Fund private holder | 00:09:00 | Private balance `42` observed |
| Positive PPE admit | 00:03:14 | Applied; nullifier recorded |
| Duplicate admit | 00:00:24 | Rejected during proving with `BA206 DuplicateNullifier` |
| Insufficient-balance rejection | 00:00:03 | Rejected during proving with `BA201 ThresholdMismatch` |
| Total | 00:13:16 | Passed with `RISC0_DEV_MODE=0` |

## LEZ Program Cycle Benchmark

Command:

```sh
LOGOS_LEZ_REPO=/path/to/logos-execution-zone-latest \
  scripts/benchmark-lez-cycles.sh
```

Canonical run:

```text
.demo-runs/cycle-bench/20260713T025856Z/benchmark.json
.demo-runs/cycle-bench/20260713T025856Z/report.md
```

Environment:

| Field | Value |
| --- | --- |
| LEZ ref | `1b4d8fbcf4caa1448473fd99b4ac9d386abf3bc3` |
| RISC Zero | `3.0.5` |
| Samples | 10 timed executions after one warmup |
| Metric | RISC Zero `SessionInfo::cycles()` user cycles |

Results:

| Operation | User cycles | Segments | Best | Mean |
| --- | ---: | ---: | ---: | ---: |
| `register_presenter` | 113,667 | 1 | 28.081 ms | 29.395 ms |
| `init_gate` | 173,308 | 1 | 27.486 ms | 29.453 ms |
| `admit_empty_gate` | 234,496 | 1 | 28.192 ms | 28.787 ms |
| `admit_after_10` | 385,290 | 1 | 30.703 ms | 32.609 ms |

The rejected duplicate path returned deterministic error `BA206`. Its measured
mean wall time was 29.076 ms, but no cycle count is reported: the RISC Zero
executor returns the guest failure before exposing `SessionInfo`. Reporting a
made-up or successful-path cycle count for that rejection would be misleading.

These values are deterministic guest execution cycles for the current
deployable Workable gate program. They are the closest metric exposed by the
current LEZ executor and match the upstream benchmark convention. They are not
labeled as network CU because the wallet/sequencer RPC does not expose a
per-transaction CU field.

## Spike 10 Direct Receipt Verification

Spike 10 measures explicit cryptographic receipt verification inside a RISC
Zero guest without the assumptions channel:

| Receipt | Bytes | User cycles | Segments | LEZ 32M limit |
| --- | ---: | ---: | ---: | --- |
| Succinct | 223,191 | 313,056,015 | 315 | rejected |
| Groth16 | 470 | 162,362,189 | 167 | rejected |

Both positive receipts verified with `RISC0_DEV_MODE=0` and both modified
journals were rejected. Groth16 removes the transaction-size concern, but its
verifier still consumes about 4.8 times the current public LEZ budget. Full
reproduction details are in
`spikes/spike-10-direct-receipt-verifier/README.md`.

## Remaining Performance Evidence

The LP requires compute-unit documentation for on-chain operations. The current
wallet/RPC path does not expose per-transaction CU data. The current gate guest
cycles are now documented; the remaining work is:

- Confirm whether evaluators accept RISC Zero user cycles as the current LEZ
  equivalent for the LP's CU documentation requirement.
- Capture per-transaction chain CU for `register_presenter`, `init_gate`,
  `admit`, and duplicate rejection if a future RPC/runtime exposes it.
- Add equivalent cycle or CU instrumentation for the final accepted on-chain
  path, including the PPE-native candidate if selected.
- Re-run Spike 09 without `SKIP_BUILD=1` when you want build timing included in
  the canonical evidence run.
- Record inclusion/finality timing on devnet/testnet separately from local
  wall-clock script timing.
- Re-run proof generation on a clean machine and record hardware details.
- Capture final narrated demo timings with `RISC0_DEV_MODE=0`.
- If LEZ later supports direct public verification of external RISC Zero
  receipts, benchmark that path separately from the current Workable gate-ledger
  model.
