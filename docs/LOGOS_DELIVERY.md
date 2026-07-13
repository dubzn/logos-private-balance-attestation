# Logos Delivery Integration

Last updated: 2026-07-13.

LP-0005 requires the off-chain proof to be transmitted over Logos Messaging and
verified locally by a recipient. The repository now has two transport surfaces:

| Surface | Status | Purpose |
| --- | --- | --- |
| CLI local JSON transport | Tested | Deterministic clean-room proof-message flow for CI and local smoke tests. |
| Basecamp `delivery_module` transport | Tested across two Basecamp instances | Real Logos Delivery module path for user-facing send/receive. A real-prover message was chunked, transferred, reassembled, hash-checked, and verified locally by the receiver. |

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
delivery_module.url = "github:logos-co/logos-delivery-module/c21ffb83b2b891843de9a940dd60e5e56c8803de";
```

This commit is newer than the `v0.1.3` tag and builds against
`logos-cpp-sdk 0.2.0`. That SDK release marshals provider events onto the
owner thread. The older Delivery build could emit Waku connection events from
an FFI worker thread, race the Qt Remote Objects reply, and make a successful
`subscribe` call appear to time out after 20 seconds.

### Temporary SDK Compatibility Patch

The current SDK cdylib generator emits an empty tagged value for every `bstr`
event argument. Delivery still sends and propagates the full chunk, but the
consumer sees an empty `messageReceived` payload. The app therefore pins the
official SDK at `d12a7bbb45d7d05f003b5d746a6c4dbc9df28315` through:

```text
apps/basecamp/nix/logos-cpp-sdk-patched/
```

The patch makes the generated event sidecar encode the actual byte argument
using the protocol's canonical `{"_bytes":"<base64url>"}` representation. It
does not change Delivery's network message limit, topic, or public API. Both
the Basecamp module and its `delivery_module` dependency are built with this
same SDK input. The patch should be deleted when the equivalent upstream fix
is released. The SDK issue is tracked at
[`logos-cpp-sdk#99`](https://github.com/logos-co/logos-cpp-sdk/issues/99).

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
or build `apps/basecamp#delivery-install` and copy that install tree into
`<user-dir>/modules/delivery_module`. Building the Delivery input directly
would bypass the pinned SDK compatibility patch.

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
     /lp0005/1/proof/json
     ```

   - press **Create node**
   - press **Subscribe**

7. In the sender Delivery panel:

   - keep `Preset` as `logos.test`
   - keep `Mode` as `Core`
   - keep the default topic:

     ```text
     /lp0005/1/proof/json
     ```

   - press **Create node**
   - if Delivery reports that the context is already initialized, keep going;
     the app reuses that node for the current Basecamp session
   - press **Subscribe**
   - press **Send proof**
   - for real `RISC0_DEV_MODE=0` receipts, the UI splits the public
     `proof-message.json` into Delivery chunks under the 150 KiB default
     message limit and logs each chunk request id

8. Receiver-side check:

   - wait for `messageReceived chunk 1/N` through `N/N`; the entries must report
     non-empty chunk data
   - wait for the Delivery log to show the full payload SHA-256 was reassembled
   - wait for `Delivery Msg` to show the reassembled proof message
   - confirm the receiver Delivery card shows `Message: received`
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

Do not publish `witness.json`. Delivery chunks carry only the public proof
envelope wrapped in the proof-message JSON. The receiver validates the
reassembled payload with a SHA-256 digest before running `message-verify`.

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
- The Basecamp backend wires `createNode`, `start`, `subscribe`, chunked `send`,
  `messageReceived`, `messageSent`, `messagePropagated`, and `messageError`.
- A two-instance run on 2026-07-13 confirmed node connectivity, subscription,
  and transfer of a 1,323,577-byte real-prover proof message in 17 Delivery
  chunks. The receiver accepted out-of-order chunks, reassembled SHA-256
  `f042487460a8485f144d0735577d4163ee63cde38860f4a270da07462b63a075`,
  and persisted the complete public message.
- `balance-attest message-verify` accepted the reassembled receiver artifact
  with `status: ok`, threshold `1`, and the expected context and presenter.
- The compatibility patch builds in both the SDK test flake and the real
  Delivery module. Upstream removal is tracked in `logos-cpp-sdk#99`.
- Final submission still needs a recorded two-instance Delivery run.
