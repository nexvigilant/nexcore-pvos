# nexcore-pvos

The Pharmacovigilance Operating System (PVOS) substrate for the NexVigilant platform. This crate provides the low-level "kernel" services required to execute safety-critical PV workflows, including syscalls, driver management, and state machine transitions.

## Intent
To provide a robust, isolated environment for the execution of PV algorithms and safety rules. It abstracts hardware and OS-level concerns into PV-specific primitives like `syscall`, `bus`, and `fsm` (Finite State Machine).

## T1 Grounding (Lex Primitiva)
Dominant Primitives:
- **ς (State)**: The core primitive for the FSM-based kernel and session lifecycle.
- **μ (Mapping)**: Maps high-level PV commands to low-level kernel operations (syscalls).
- **→ (Causality)**: Manages the sequence of event triggers and feedback loops.
- **∂ (Boundary)**: Enforces process-level isolation and security boundaries (WAL, Signature).
- **π (Persistence)**: Manages the Write-Ahead Log (WAL) and durable kernel state.

## Core Kernel Services
- **Syscalls**: Standardized interface for interacting with the PVOS kernel.
- **FSM Engine**: High-performance finite state machine for tracking case progression.
- **Event Bus**: Low-latency internal messaging for kernel components.
- **WAL (Write-Ahead Log)**: Ensures ACID compliance for safety-critical state changes.
- **Supervisor**: Manages agent and process lifecycles with automated recovery.

## SOPs for Use
### Interacting with the Kernel
```rust
use nexcore_pvos::kernel::Kernel;
let mut kernel = Kernel::boot()?;
let result = kernel.syscall(Command::DetectSignal(payload))?;
```

### Defining a State Transition
Transitions are handled via the `fsm` module, requiring an explicit `Transition` record to ensure traceability and auditability.

## Key Components
- **Kernel**: The central coordinator for all PVOS services.
- **Typestate**: Compile-time verification of valid kernel states.
- **Backpressure**: Flow control mechanism for high-volume signal streams.

## License
Proprietary. Copyright (c) 2026 NexVigilant LLC. All Rights Reserved.
