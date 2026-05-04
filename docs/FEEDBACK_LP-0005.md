# Feedback de revisión — LP-0005 (Private Balance Attestation)

**Fecha:** 2026-04-30
**Branch:** master (último commit `c374831 Add balance circuit spike`)
**Alcance:** auditoría del estado actual del repo contra los Success Criteria de `LP-0005.md`.

---

## 1. Resumen ejecutivo

El repo está **al final de la fase de spikes**, no al principio de la implementación. Hay 4 spikes funcionales, 14 scripts de infraestructura y 10 documentos de diseño. **No hay todavía nada del producto final**: ni `crates/`, ni `methods/guest/`, ni `lez/verifier-program/`, ni `apps/basecamp/`, ni `examples/`. Lo que hay es honesto y prudente para esta fase, pero hay **deriva entre lo documentado y lo validado** que conviene cerrar antes de empezar M1.

El veredicto rápido:

- **Spike 03 (balance-circuit)** ya valida la pieza criptográfica más cara (formato real de commitment LEZ + Merkle path live + threshold check dentro de RISC Zero). Eso es *el* logro de la fase.
- **Spike 0A (receipt verification dentro de un guest público)** falló — y la doc no lo refleja consistentemente.
- **Spike 0B (recursive/native verifier)** está documentado pero nunca se ejecutó — es un fantasma.
- **Spike 0C (private-execution gate)** pasó, pero su modelo no produce un proof portable y por sí solo no satisface el wording de LP-0005.
- **Falta absolutamente todo lo de presenter binding y nullifier scheme** — son requerimientos explícitos de LP-0005 (líneas 29-30 y 74-76), y no aparecen en ningún spike ni doc concreta.

---

## 2. Lo que está implementado

### Spikes que corren end-to-end contra sequencer real
- `spikes/receipt-verification` — guest público que llama `env::verify` con image_id hardcodeado a ceros. Sirvió para descartar la ruta directa.
- `spikes/private-balance-gate` — guest privado que comprueba `holder.balance >= threshold` y escribe un marker en una cuenta pública. **No** produce proof portable.
- `spikes/membership-proof` — runner de diagnóstico que llama `getProofForCommitment` real y recompone la raíz Merkle con `nssa_core::compute_digest_for_path`.
- `spikes/balance-circuit` — guest RISC Zero independiente que reconstruye el commitment con `nssa_core::Commitment::new` (formato real LEZ), verifica el Merkle path, asserta el threshold y commitea un journal `BalanceAttestationJournal { version, threshold, commitment_root, context_id, commitment, proof_index, proof_depth }`. Tiene cobertura de modos de fallo (`fixture-below-threshold`, `fixture-bad-root`, `live-below-threshold`).

### Infraestructura
- 14 scripts en `scripts/`: install/build/run por spike, con un `check-risc0-version.sh` que pinea contra el LEZ checkout.
- Modular Test Plan que separa por capas (Layer 0 entorno → Layer 8 E2E).
- Fixtures offline reproducibles para Spike 03 sin tocar sequencer.

### Documentación
- 10 docs en `docs/` cubriendo arquitectura, error codes, IDL draft, plan de implementación, setup local, plan de tests, checklist del prize, notas de referencia, plan de spikes y modelo de seguridad.
- `REFERENCE_NOTES.md` actúa como cuaderno de laboratorio con resultados de spikes datados y account IDs reales.

---

## 3. Lo que falta (mapeado a LP-0005)

| LP-0005 requirement | Estado | Dónde debería vivir |
|---|---|---|
| Circuito production que prueba `balance >= N` con context binding y presenter binding | **No existe** | `methods/guest/` |
| Crate `attestation-core` (tipos compartidos, formato de proof, error codes) | **No existe** | `crates/attestation-core/` |
| Crate `attestation-prover` (cliente que toma cuenta privada → proof) | **No existe** | `crates/attestation-prover/` |
| Crate `attestation-verifier` (verificación off-chain) | **No existe** | `crates/attestation-verifier/` |
| CLI | **No existe** | `crates/attestation-cli/` |
| Programa LEZ verificador on-chain | **No existe** | `lez/verifier-program/` |
| Integración Logos Messaging | **No existe** | — |
| Basecamp app GUI | **No existe** | `apps/basecamp/` |
| 3 integraciones de aplicaciones (al menos una por equipo externo) | **No existe ni partner identificado** | `examples/` |
| SPEL/IDL artifact real | **No existe** | — |
| CI verde | **No existe** | — |
| Demo video con `RISC0_DEV_MODE=0` | **No existe** | — |
| Benchmarks (proof gen time, on-chain verification gas) | **No existe** | — |
| Presenter binding (clave de quien presenta) | **No existe en ningún spike** | circuito + envelope |
| Nullifier scheme (anti-replay por contexto) | **No existe** | circuito |
| Context binding real (no sólo passthrough) | **No existe** — `context_id` se publica pero no se usa | circuito |

**Resultado**: el camino más caro (probar que el commitment LEZ se puede reconstruir y verificar membership dentro de un guest RISC Zero con proof tiempo razonable) ya está cubierto. Lo que falta es **toda la capa de producto**: envelope, presenter binding, nullifier, programa LEZ, SDK, integraciones, demo. La fase de spikes desbloquea M1, no la sustituye.

---

## 4. Cosas que me gustan (mantener)

1. **Spike 03 usa `nssa_core::Commitment::new` y `compute_digest_for_path` reales** (`spikes/balance-circuit/lez/guest/src/bin/balance_attestation_spike.rs:36-41`). No hay reimplementación del formato de commitment — se delega al helper LEZ. Es exactamente la lección de la submission previa que `REFERENCE_NOTES.md:26-41` documenta.

2. **`REFERENCE_NOTES.md` como cuaderno de laboratorio**. Lleva lessons learned de la submission anterior (líneas 26-41), resultados de spikes con fechas y account IDs (líneas 163-340), y refuse a sobre-vender Spike 01 ("This does not yet satisfy the LP-0005 wording by itself", `REFERENCE_NOTES.md:228-240`). Esa honestidad es oro.

3. **Orden de spikes guiado por riesgo**, no por entusiasmo. `RISK_SPIKES.md` define stop conditions reales (líneas 217-224) — no hedging optimista.

4. **Cobertura de fallos en Spike 03**. `fixture-below-threshold`, `fixture-bad-root`, `live-below-threshold` (`spikes/balance-circuit/lez/runner/src/bin/prove_balance_attestation_spike.rs:61-104`). Validar el rechazo es tan importante como validar la aceptación.

5. **Detección temprana del prefijo/domain del commitment** (`docs/ARCHITECTURE.md:52-66`). Esa es la trampa que mata integraciones; capturarlo antes de escribir código es exactamente lo correcto.

6. **`LOCAL_SETUP.md`** captura gotchas operativos reales (cadencia de bloque de 15 s, `RISC0_DEV_MODE=1` debe coincidir entre wallet y sequencer en spike 01) — son cosas que no se infieren leyendo código.

7. **`spike-01-demo-private-gate.sh`** orquesta el camino feliz **y** un negativo, asertando que el rechazo aparece — no sólo que el positivo pasa.

8. **`check-risc0-version.sh`** previene exactamente el modo de falla que mató la submission anterior (versión RISC Zero divergente). Pinearlo contra el `Cargo.toml` de LEZ es la decisión correcta.

9. **Modular Test Plan** por capas, en vez de saltar a un E2E gigante.

---

## 5. Problemas (corregir antes de M1)

### 5.1 Críticos — bloquean LP-0005, no spike phase

**P1. No hay presenter binding en ningún spike ni doc concreta.** LP-0005 línea 30 y la sección "Proof Forwarding" (líneas 74-76) lo piden explícitamente. Spike 03 no tiene `pk_presenter`, no firma un challenge, no compromete una clave en el journal. Cualquier holder del receipt puede replay. `SECURITY_MODEL.md:124-127` lo menciona como "escalation note" pero no decide. **Decisión necesaria antes de M1**: qué clave usa el presenter (la signing key del wallet? una key derivada por contexto?), si va dentro del circuito o fuera (envelope con firma sobre el journal hash), y cómo el verifier la chequea on-chain vs off-chain.

**P2. No hay nullifier scheme.** Spike 03 no deriva ni publica ningún nullifier. `SECURITY_MODEL.md:65-77` propone un esquema con domain prefix pero no está implementado. Sin nullifier no hay anti-replay por contexto on-chain (cualquiera puede pasar el mismo gate dos veces).

**P3. `context_id` es passthrough, no binding.** En Spike 03 el `context_id` se lee, se publica en el journal y nunca se usa en ningún chequeo, hash ni nullifier (`balance_attestation_spike.rs:43-51`). Un verifier que lee el journal ve un tag de 32 bytes sin atadura semántica al proof. Esto es exactamente lo opuesto de "context binding".

**P4. Spike 0A falló y la doc no lo refleja consistentemente.** `REFERENCE_NOTES.md:163-190` registra que `env::verify` no funciona en LEZ public execution sin un canal de assumptions. Pero `ARCHITECTURE.md:234-247` y `docs/IDL_DRAFT.md` siguen tratando direct receipt verification como opción primaria, y el `claim_access` de IDL_DRAFT está modelado sobre ese supuesto. **Si 0A está descartado, IDL_DRAFT.md no describe un programa viable.**

**P5. Spike 0B (recursive/native verifier) es un fantasma.** `RISK_SPIKES.md:50-71` lo describe, `IMPLEMENTATION_PLAN.md` M0.5 paso 3 lo lista, `ARCHITECTURE.md:244` lo menciona como opción 2 — pero **no hay script, no hay directorio, no hay resultado**. Hay que ejecutarlo o tacharlo. Decir que "vamos a explorar 0A, 0B y 0C" cuando 0B nunca se intentó es deuda informacional.

### 5.2 Importantes — coherencia de docs vs realidad

**P6. Dos fórmulas distintas de `context_id`.** `ARCHITECTURE.md:146-154` incluye `image_id` en la derivación; `SECURITY_MODEL.md:49-56` no. Es divergencia sustantiva, no typo. Decidir antes de codear.

**P7. `init_gate` lista un error code que no aplica.** `IDL_DRAFT.md:107` pone `BA502 UnauthorizedPresenterAccount` en `init_gate`, pero esa instrucción toma un `admin`, no un presenter. Pertenece sólo a `claim_access`.

**P8. `PRIZE_CHECKLIST.md` está desactualizado.** Filas que deberían ser `in-progress` por mérito de Spike 02/03 (formato de commitment LEZ, membership proof) siguen en `planned`. Eso le quita valor al checklist como termómetro de progreso.

**P9. El journal de Spike 03 publica el `commitment` (leaf hash).** `balance_attestation_spike.rs:43-51`. LP-0005 línea 28 pide "verifiable without revealing … account identity". El leaf es linkeable al árbol Merkle on-chain por un observador pasivo. No es necesariamente un bug, pero **hay que decidir explícitamente** si el leaf se publica o se reemplaza por una root + nullifier sin leaf, y documentar el trade-off en `SECURITY_MODEL.md`.

### 5.3 Calidad de la infra de spikes

**P10. Todos los scripts default `RISC0_DEV_MODE=1`.** No hay todavía ni un solo run con `RISC0_DEV_MODE=0`. LP-0005 exige el demo final con `=0` (línea 60). Dejar la primera medición real para el final es donde aparecen las sorpresas (proof time, memoria, version drift). Sugerencia: un `make spike-03-prod` que corra el camino feliz con `=0` ya, aunque sea slow, para tener una baseline.

**P11. Los install scripts mutan el árbol de LEZ in-place.** `spike-XX-install-lez-sources.sh` copia archivos a `$LEZ_REPO/examples/program_deployment/...` y appendea deps a Cargo.toml de LEZ con un grep frágil. No es idempotente entre spikes (corres 00, después 01, y ya hay deps duplicadas/superpuestas). No hay cleanup. Para una review externa esto es inejecutable sin un LEZ checkout dedicado.

**P12. Detección de marker por `od` + grep hexadecimal.** `spike-00-run-direct-receipt-gate.sh:70-72`, `spike-01-run-private-gate.sh:55-65`. Cualquier cambio en el formato de output de `wallet account get` rompe el assert. Para infra throwaway alcanza, pero **no debe migrar a M1**.

**P13. `LEZ_REPO` por defecto es `$HOME/logos/src/logos-execution-zone`** y el layout real del usuario es `~/Desktop/logos/...`. Cada invocación necesita exportar la var. Cambiar el default o documentarlo arriba en el README.

**P14. Las fixtures de Spike 03 son obvias y constantes** (npk all-7s, balance 42, sibling bytes 0x11/0x22/0x33/0x44). Está bien para un spike, pero conviene marcar explícitamente en el código (no sólo en el README) que esos valores son sintéticos y no representan un account real.

**P15. Spike 0 nunca compone con Spike 03.** El image_id en `receipt_gate.rs:7` es `0x00…00` y el `expected_journal_words` que el runner pasa es `Vec::new()` (`run_receipt_gate.rs:47-50`). No hay un test que tome el receipt producido por Spike 03 y lo verifique con Spike 0 — que es exactamente lo que el on-chain path de LP-0005 necesita. Aunque 0A esté descartado por razones de assumptions channel, la composición conceptual debería estar al menos en un stub.

### 5.4 Menores

**P16. Sin `Cargo.toml` en este repo.** Los spikes dependen de un workspace LEZ externo. Es razonable hoy, pero significa que el repo no es self-contained ni para `cargo check`. M1 debería traer un workspace propio antes de copiar nada más a LEZ.

**P17. Sin CI.** Ningún job corre nada de esto. LP-0005 requiere CI verde (línea 57). Cuanto antes se introduzca un job (aunque sólo corra `check-risc0-version.sh` + `shellcheck` sobre los scripts), mejor.

**P18. Verificación de fallos por substring matching.** `prove_balance_attestation_spike.rs:164-175` chequea que el error del prover contenga un substring. Frágil al refactor de mensajes. Para Spike alcanza; para production tests no.

**P19. No hay socio externo identificado** para la integración M8 / Success Criteria línea 34. Buscarlo desde ya, no en M8.

---

## 6. Inconsistencias documentales (lista corta)

1. ARCHITECTURE.md vs SECURITY_MODEL.md: fórmulas distintas de `context_id`. **(P6)**
2. ARCHITECTURE.md / IDL_DRAFT.md tratan 0A como vivo; REFERENCE_NOTES.md dice que falló. **(P4)**
3. RISK_SPIKES.md / IMPLEMENTATION_PLAN.md / ARCHITECTURE.md mencionan 0B como ruta validable; no se ejecutó. **(P5)**
4. PRIZE_CHECKLIST.md no refleja avances de Spike 02/03. **(P8)**
5. IDL_DRAFT.md `init_gate` usa error code de presenter. **(P7)**
6. LOCAL_SETUP.md y MODULAR_TEST_PLAN.md describen comandos `attestation-cli` que no existen, mezclando "diseñado" con "probado".
7. RISK_SPIKES.md no tiene una tabla resumen de status por spike (0A=fail, 0B=not run, 0C=pass, 02=pass, 03=pass). Conviene agregarla arriba.

---

## 7. Recomendación de prioridades antes de empezar M1

1. **Cerrar el gap 0A/0B/0C en docs** (1 día). Reescribir ARCHITECTURE.md sección "On-chain proof verification" alrededor del modelo 0C, marcar 0A como descartado con link a REFERENCE_NOTES, ejecutar 0B o tacharlo formalmente. Reescribir IDL_DRAFT.md para que `claim_access` describa el flujo bajo 0C (ya no recibe un receipt RISC Zero anidado para verificar, sino un proof producido por private execution).
2. **Decidir presenter binding y nullifier scheme** (P1, P2) antes de codear nada de M1, y plasmarlo en `SECURITY_MODEL.md` como decisiones (no como notas).
3. **Unificar `context_id`** (P6) y bindearlo de verdad en el siguiente revision del circuito (Spike 04 o directamente M3 production circuit): tiene que ir hashed dentro del nullifier o de un campo journal que el verifier compara contra un tag esperado.
4. **Tomar una baseline con `RISC0_DEV_MODE=0`** para Spike 03 — proof time, memoria, tamaño del receipt. No esperar al demo.
5. **Inicializar el workspace propio del repo** (`Cargo.toml` raíz, `crates/attestation-core/`) e introducir CI mínimo. M1 empieza ahí.
6. **Buscar el partner externo** para la integración M8. Es el item de mayor lead time y no depende del código.

---

## 8. Observación final

La fase de spikes hizo lo que tenía que hacer: validó la pieza criptográfica más cara (Spike 03), validó la disponibilidad del membership proof live (Spike 02), descartó una ruta on-chain inviable (Spike 0A) y registró otra como fallback aceptable (Spike 0C). Eso es buen trabajo de descubrimiento.

El riesgo ahora no es técnico, es de **deriva**: hay docs escritas antes de los spikes que no se actualizaron a la luz de los resultados. Si M1 arranca contra esa documentación, va a construir un programa LEZ basado en un modelo de verificación (0A) que ya sabemos que no funciona, y va a omitir presenter binding y nullifier scheme porque los docs concretos no los obligan. Antes de escribir la primera línea de `attestation-core`, conviene hacer una pasada de sincronización docs↔realidad, decidir las dos cuestiones criptográficas abiertas (presenter binding, nullifier) y confirmar 0B o enterrarlo.

Si esas tres cosas se cierran, el proyecto está en buena forma para entrar a M1 con un ETA realista hacia submission.
