# AI Guidance — nexcore-pvos

Pharmacovigilance Operating System (PVOS) kernel and substrate.

## Use When
- Implementing low-level system services for PV execution.
- Managing long-running safety state machines (FSMs).
- Interfacing with durable logs (WAL) or internal event buses.
- Implementing "Kernel Mode" operations that require high isolation.

## Grounding Patterns
- **Syscall Pattern**: All external interactions with the PVOS core should go through the `syscall()` interface to ensure proper auditing and boundary enforcement.
- **Typestate Safety**: Use the `typestate` module pattern to prevent illegal state transitions at compile-time.
- **T1 Primitives**:
  - `ς + π`: Root primitives for stateful, durable kernel operations.
  - `μ + →`: Root primitives for command mapping and event causality.

## Maintenance SOPs
- **WAL Invariant**: Never modify the internal state directly; always append to the Write-Ahead Log (WAL) first to ensure crash-recovery integrity.
- **Supervisor Policy**: Any new critical service MUST be registered with the `Supervisor` to enable automatic health-checks and restart logic.
- **Isolation**: Process boundaries MUST be respected. Use the `isolation` module for cross-tenant operations.

## Key Entry Points
- `src/kernel.rs`: The main kernel implementation and boot logic.
- `src/syscall.rs`: Standardized command and response definitions.
- `src/fsm_transition.rs`: State machine logic for case lifecycles.
- `src/lib.rs`: Facade and common re-exports.
