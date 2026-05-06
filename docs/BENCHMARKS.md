# Benchmarks

Last updated: 2026-05-06

This document records measured runs for LP-0005 without turning local numbers
into testnet claims. The current benchmark source is a full local E2E run against
a local LEZ sequencer, local wallet private state, and the real
`getProofForCommitment` path.

## Scope And Caveats

- Run source:
  `.demo-runs/local-full/20260505T211442Z/run.json`
- Proof source:
  `.demo-runs/local-full/20260505T211442Z/proof/run.json`
- Gate source:
  `.demo-runs/local-full/20260505T211442Z/gate/run.json`
- `RISC0_DEV_MODE=0`.
- The proof path uses real wallet private state and the real wallet/sequencer
  membership-proof path.
- The gate path is the current Workable model: host-side proof verification via
  `attestation-verifier`, followed by a LEZ gate-ledger transaction that records
  and deduplicates the context nullifier.
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
| Gate account | `Public/Fi1rvig7zFFB3Jy3iRi9avTdLo9YRyrkddFJSDySQVbt` |
| Presenter account | `Public/CvacJ5b34Xv16UWtgxXQj5Pk9QWCtHxMVvukctm63BLo` |
| Context nullifier | `72dab09ed8ed94ae43c5996c470e531cf916c46fcf67a35724a61d0867f38fb6` |
| Nullifier count in gate data | `1` |
| Duplicate admit | `not-applied` |

Phase timing:

| Phase | Duration | Output |
| --- | ---: | --- |
| Proof | 00:02:20 | `.demo-runs/local-full/20260505T211442Z/proof/report.md` |
| Gate | 00:02:49 | `.demo-runs/local-full/20260505T211442Z/gate/report.md` |
| Total | 00:05:09 | `.demo-runs/local-full/20260505T211442Z/report.md` |

## Proof Benchmark

This phase builds a witness from private wallet state, proves
`balance >= threshold`, and verifies the public envelope locally.

| Step | Duration | Notes |
| --- | ---: | --- |
| Build | 00:00:44 | Workspace/guest build as invoked by the script. |
| Wallet health | 00:00:00 | Local wallet/sequencer health check. |
| Build witness | 00:01:09 | Reads private wallet state and calls the real `getProofForCommitment` path. |
| Prove | 00:00:23 | RISC Zero proof generation with `RISC0_DEV_MODE=0`. |
| Verify | 00:00:03 | Local off-chain verifier over the public envelope. |
| Total | 00:02:19 | Proof script total from `proof/run.json`. |

Proof metadata:

| Metric | Value |
| --- | --- |
| Threshold | `1` |
| Proof index | `4` |
| Proof depth | `4` |
| Public envelope size | `1,323,421` bytes |
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
| Build | 00:01:41 | CLI, deployable LEZ program, and runner build work. |
| Wallet health + deploy | 00:00:15 | Local wallet health check and program deployment path. |
| Create accounts | 00:00:00 | Fresh public accounts for gate, presenter, and admins. |
| Dry run | 00:00:01 | Prepared register/init/admit commands without execution. |
| Register presenter | 00:00:01 | Stores presenter pubkey in the presenter account. |
| Init gate | 00:00:00 | Initializes gate account with expected context and threshold. |
| Admit | 00:00:05 | Host-verifies envelope, submits admit, observes nullifier in gate data. |
| Duplicate admit | 00:00:46 | Replays same admit and confirms duplicate is not applied after settle. |
| Total | 00:02:49 | Gate script total from `gate/run.json`. |

Local transaction hashes captured by the run:

| Operation | Transaction hash |
| --- | --- |
| Register presenter | `39a60a9c7c65d03f0bed95fd9a7415331c5df1fbf134b5c04ac90a3d3b24d5ee` |
| Init gate | `41967c2bd902ee247694736f10aecfad54fcef91ac776b157eb0faf35e033d0e` |
| Admit | `52edfe4393b023f7a0f5a6857bfc8e9b1fb8903a8e21b80595297b6e01aad108` |
| Duplicate admit | `fd8fbb5f87aa1ba793acafcb158fbdfc4c87152eaffd3b7642255971d82063f2` |

The duplicate transaction was submitted, but the final gate state still contains
one copy of the context nullifier.

## What Still Needs Testnet/CU Measurement

The LP requires compute-unit documentation for on-chain operations. The current
local run does not provide CU data, so these remain open:

- Deploy the verifier program on LEZ devnet/testnet and record the deployed
  program id.
- Measure CU cost for `register_presenter`, `init_gate`, `admit`, and rejected
  duplicate admit if the chain exposes per-transaction CU metrics.
- Record inclusion/finality timing on devnet/testnet separately from local
  wall-clock script timing.
- Re-run proof generation on a clean machine and record hardware details.
- Capture final narrated demo timings with `RISC0_DEV_MODE=0`.
- If LEZ later supports direct public verification of external RISC Zero
  receipts, benchmark that path separately from the current Workable gate-ledger
  model.
