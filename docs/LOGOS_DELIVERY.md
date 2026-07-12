# Logos Delivery Integration

Last updated: 2026-07-11.

LP-0005 requires the off-chain proof to be transmitted over Logos Messaging and
verified locally by a recipient. The repository now has two transport surfaces:

| Surface | Status | Purpose |
| --- | --- | --- |
| CLI local JSON transport | Tested | Deterministic clean-room proof-message flow for CI and local smoke tests. |
| Basecamp `delivery_module` transport | Builds / two-instance QA ready | Real Logos Delivery module path for user-facing send/receive. |

The message payload is intentionally the same in both cases: a V1 proof message
wrapping the public attestation envelope. Transport changes should not change
the proof-message bytes or verifier behavior.

## Basecamp Delivery Path

The Basecamp module declares the real Delivery dependency:

```json
"dependencies": ["delivery_module"]
```

The Nix module pins:

```nix
delivery_module.url = "github:logos-co/logos-delivery-module/v0.1.3";
```

Build and inspect the package:

```sh
scripts/check-basecamp-package.sh
```

This verifies that the generated install tree contains the `delivery_module`
dependency in both `manifest.json` and `metadata.json`.

For a single local Basecamp launch, use the repository helper:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

It installs the UI plugin and the Delivery module into the same user directory:

```text
<user-dir>/plugins/balance_attestation/
<user-dir>/modules/delivery_module/
```

If Basecamp shows a **Missing Dependencies** dialog for `delivery_module`, the
Delivery module was not installed into that user directory. Rerun the launcher
or copy `github:logos-co/logos-delivery-module/v0.1.3#install` into
`<user-dir>/modules/delivery_module`.

For the two-instance Delivery walkthrough, prepare both Basecamp user dirs first:

```sh
scripts/prepare-basecamp-delivery-pair.sh --reset --real-prover
```

The script builds/inspects the `balance_attestation` package, installs the real
`delivery_module` into both user dirs, and writes:

```text
.demo-runs/basecamp-delivery-pair/<timestamp>/launch-sender.sh
.demo-runs/basecamp-delivery-pair/<timestamp>/launch-receiver.sh
.demo-runs/basecamp-delivery-pair/<timestamp>/report.md
```

## Manual Two-Instance QA

Use this when recording the off-chain Messaging portion of the final demo.

1. Start the local LEZ sequencer and prepare a private account.

2. Prepare the two user dirs:

   ```sh
   scripts/prepare-basecamp-delivery-pair.sh --reset --real-prover
   ```

3. Open two terminals and run the generated launchers:

   ```sh
   .demo-runs/basecamp-delivery-pair/<timestamp>/launch-sender.sh
   .demo-runs/basecamp-delivery-pair/<timestamp>/launch-receiver.sh
   ```

4. In both Basecamp windows, load `balance_attestation` from
   `Modules -> UI Modules`.

5. Generate and verify a proof from the sender instance:

   - Preflight
   - Generate proof
   - Verify envelope

6. In the receiver Delivery panel:

   - keep `Preset` as `logos.test`
   - keep `Mode` as `Core`
   - keep the default topic:

     ```text
     /lp0005-balance-attestation/1/proof-envelope/json
     ```

   - press **Create node**
   - press **Subscribe**

7. In the sender Delivery panel:

   - keep `Preset` as `logos.test`
   - keep `Mode` as `Core`
   - keep the default topic:

     ```text
     /lp0005-balance-attestation/1/proof-envelope/json
     ```

   - press **Create node**
   - if Delivery reports that the context is already initialized, keep going;
     the app reuses that node for the current Basecamp session
   - press **Subscribe**
   - press **Send proof**

8. Receiver-side check:

   - wait for `Delivery Msg` to show the received proof message
   - press **Verify received**

Expected receiver result:

```json
{
  "status": "ok"
}
```

The receiver verifies locally with `balance-attest message-verify`. The received
message is saved as:

```text
.demo-runs/basecamp*/delivery/proof-message.json
```

Do not publish `witness.json`. Delivery messages carry only the public proof
envelope.

## CLI Fallback

For deterministic CI-friendly smoke tests:

```sh
./demo.sh --messaging --real-prover
```

This does not use the Delivery network. It exercises the same payload shape and
local verifier/admission semantics through `attestation-messaging`.

## Current Evidence

- `scripts/check-basecamp-package.sh` passes with the real `delivery_module`
  dependency.
- `scripts/prepare-basecamp-delivery-pair.sh` prepares sender/receiver
  Basecamp user dirs with both the UI plugin and `delivery_module` installed.
- The Basecamp backend wires `createNode`, `start`, `subscribe`, `send`,
  `messageReceived`, `messageSent`, `messagePropagated`, and `messageError`.
- Final submission still needs a recorded two-instance Delivery run.
