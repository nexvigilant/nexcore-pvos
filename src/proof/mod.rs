//! # PVST — Formal Proofs
//!
//! Formal verification types for PVST state machines using
//! conservation laws and type-level encoding.
//!
//! ## Conservation Laws Verified
//!
//! | Law | Name | Invariant |
//! |-----|------|-----------|
//! | L3 | Single State | Machine in exactly 1 state at any time |
//! | L4 | Non-Terminal Flux | Non-terminal states have outgoing transitions |
//! | L11 | Structure Immutability | State count immutable after construction |
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | ς      | State       | 0.80 (dominant) |
//! | →      | Causality   | 0.10   |
//! | κ      | Comparison  | 0.10   |

pub mod conservation;

pub use conservation::{
    ConservationLaw, ConservationVerifier, L3SingleState, L4NonTerminalFlux,
    L11StructureImmutability, VerificationResult,
};
