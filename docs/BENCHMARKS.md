# Benchmarks

Last updated: 2026-05-19

This document records measured runs for LP-0005 without turning local numbers
into testnet claims. The current benchmark source is a full local E2E run against
a local LEZ sequencer, local wallet private state, and the real
`getProofForCommitment` path.

## Scope And Caveats

- Run source:
  `.demo-runs/local-full/20260508T150913Z/run.json`
- Proof source:
  `.demo-runs/local-full/20260508T150913Z/proof/run.json`
- Gate source:
  `.demo-runs/local-full/20260508T150913Z/gate/run.json`
- `RISC0_DEV_MODE=0`.
- The proof path uses real wallet private state and the real wallet/sequencer
  membership-proof path.
- The gate path is the current Workable model: host-side proof verification via
  `attestation-verifier`, followed by a LEZ gate-ledger transaction that records
  and deduplicates the context nullifier.
- Spike 09 PPE-native gate timing is included below as separate local evidence.
- These are local wall-clock timings, not LEZ devnet/testnet CU measurements.
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
| Gate account | `Public/6VtWyvdEPAwVJdGSLEH2CH6D5rnp7eaXGz5dEbhm5hvv` |
| Presenter account | `Public/AAJzVHgfDFMsY2Ugs1rGd2Wb1MvL8W1vCwJd3a3r7PEs` |
| Context nullifier | `0ff1e803775d28ff6b4830630a2ffd8b51a98e5e6b3db7a4f47f034434443872` |
| Nullifier count in gate data | `1` |
| Duplicate admit | `not-applied` |

Phase timing:

| Phase | Duration | Output |
| --- | ---: | --- |
| Proof | 00:01:48 | `.demo-runs/local-full/20260508T150913Z/proof/report.md` |
| Gate | 00:01:42 | `.demo-runs/local-full/20260508T150913Z/gate/report.md` |
| Total | 00:03:30 | `.demo-runs/local-full/20260508T150913Z/report.md` |

## Proof Benchmark

This phase builds a witness from private wallet state, proves
`balance >= threshold`, and verifies the public envelope locally.

| Step | Duration | Notes |
| --- | ---: | --- |
| Build | 00:00:02 | Workspace/guest build as invoked by the script. |
| Wallet health | 00:00:01 | Local wallet/sequencer health check. |
| Build witness | 00:01:17 | Reads private wallet state and calls the real `getProofForCommitment` path. |
| Prove | 00:00:24 | RISC Zero proof generation with `RISC0_DEV_MODE=0`. |
| Verify | 00:00:03 | Local off-chain verifier over the public envelope. |
| Total | 00:01:47 | Proof script total from `proof/run.json`. |

Proof metadata:

| Metric | Value |
| --- | --- |
| Threshold | `1` |
| Proof index | `4` |
| Proof depth | `4` |
| Public envelope size | `1,322,931` bytes |
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
| Wallet health + deploy | 00:00:15 | Local wallet health check and program deployment path. |
| Create accounts | 00:00:00 | Fresh public accounts for gate, presenter, and admins. |
| Dry run | 00:00:01 | Prepared register/init/admit commands without execution. |
| Register presenter | 00:00:05 | Stores presenter pubkey in the presenter account. |
| Init gate | 00:00:16 | Initializes gate account with expected context and threshold. |
| Admit | 00:00:15 | Host-verifies envelope, submits admit, observes nullifier in gate data. |
| Duplicate admit | 00:00:46 | Replays same admit and confirms duplicate is not applied after settle. |
| Total | 00:01:42 | Gate script total from `gate/run.json`. |

Local transaction hashes captured by the run:

| Operation | Transaction hash |
| --- | --- |
| Register presenter | `5d891e7eee1dc83c45eb9c98e641c9677b79d2d48339e86a62d7fcf4b57c1e30` |
| Init gate | `aec571de83f348aed0e61838beb9b38813121a547c6e8d95c49d10648ca1b6f2` |
| Admit | `db781510a073680fb67776859d254aca2f2b84f4157921c47abca77a1e4d2289` |
| Duplicate admit | `00f02b10abee37e36667440135c431bdf32885d50a5d77c7a3bbb1f0a91e1cb4` |

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

## What Still Needs Testnet/CU Measurement

The LP requires compute-unit documentation for on-chain operations. The current
local run does not provide CU data, so these remain open:

- Deploy the verifier program on LEZ devnet/testnet and record the deployed
  program id.
- Measure CU cost for `register_presenter`, `init_gate`, `admit`, and rejected
  duplicate admit if the chain exposes per-transaction CU metrics.
- Re-run Spike 09 without `SKIP_BUILD=1` when you want build timing included in
  the canonical evidence run.
- Record inclusion/finality timing on devnet/testnet separately from local
  wall-clock script timing.
- Re-run proof generation on a clean machine and record hardware details.
- Capture final narrated demo timings with `RISC0_DEV_MODE=0`.
- If LEZ later supports direct public verification of external RISC Zero
  receipts, benchmark that path separately from the current Workable gate-ledger
  model.
