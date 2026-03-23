//! # PVTX Audit Sealing
//!
//! Cryptographic closure of audit periods with tamper detection.
//! Once sealed, records within the scope cannot be modified (∝).
//! Seals are hash-linked into a chain for tamper evidence.
//!
//! ## Primitives
//! - ∝ (Irreversibility) — DOMINANT: seals are permanent
//! - π (Persistence) — immutable archival storage
//! - → (Causality) — hash-linked chain
//! - N (Quantity) — scope boundaries
//!
//! ## Design
//!
//! ```text
//! Seal₁ → Seal₂ → Seal₃ → ...
//!   ↓        ↓        ↓
//! Records  Records  Records
//! (Q1/24)  (Q2/24)  (Q3/24)
//! ```
//!
//! Each seal covers a scope (time period, record range) and
//! contains a hash of all records within. The seal chain
//! provides blockchain-lite tamper evidence.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// T2-P NEWTYPES
// ===============================================================

/// Unique seal identifier.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SealId(pub u64);

impl SealId {
    /// Creates a new seal ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SealId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SEAL-{:08X}", self.0)
    }
}

impl GroundsTo for SealId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// SEAL SCOPE
// ===============================================================

/// What records are included in a seal.
/// Tier: T2-P (N + ∝)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SealScope {
    /// A calendar quarter (year, quarter 1-4).
    Quarter(u32, u8),
    /// A calendar year.
    Year(u32),
    /// A specific time range (epoch start, epoch end).
    TimeRange { start_epoch: u64, end_epoch: u64 },
    /// A specific record range (start ID, end ID inclusive).
    RecordRange { start_id: u64, end_id: u64 },
    /// Custom named scope.
    Custom(String),
}

impl SealScope {
    /// Creates a quarterly scope.
    #[must_use]
    pub fn quarter(year: u32, quarter: u8) -> Self {
        Self::Quarter(year, quarter.clamp(1, 4))
    }

    /// Creates a yearly scope.
    #[must_use]
    pub fn year(year: u32) -> Self {
        Self::Year(year)
    }

    /// Human-readable description of the scope.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Quarter(year, q) => format!("Q{q}/{year}"),
            Self::Year(year) => format!("FY{year}"),
            Self::TimeRange {
                start_epoch,
                end_epoch,
            } => {
                format!("epoch:{start_epoch}-{end_epoch}")
            }
            Self::RecordRange { start_id, end_id } => {
                format!("records:{start_id}-{end_id}")
            }
            Self::Custom(name) => name.clone(),
        }
    }
}

impl GroundsTo for SealScope {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// TAMPER EVIDENCE
// ===============================================================

/// Result of tamper detection verification.
/// Tier: T2-P (∝ + ∂)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TamperVerdict {
    /// No tampering detected — hashes match.
    Clean,
    /// Tampering detected — hash mismatch.
    Tampered {
        /// Which seal index was tampered.
        seal_index: usize,
        /// Expected hash.
        expected: u64,
        /// Actual hash found.
        actual: u64,
    },
    /// Chain is broken (missing link).
    BrokenChain {
        /// Where the break occurred.
        break_index: usize,
    },
}

impl TamperVerdict {
    /// Returns true if no tampering was found.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Clean)
    }
}

impl GroundsTo for TamperVerdict {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility, LexPrimitiva::Boundary])
    }
}

// ===============================================================
// SEAL
// ===============================================================

/// A cryptographic seal — permanent closure of an audit period.
/// Tier: T2-C (∝ + π + → + N)
///
/// Once created, a seal is immutable. It contains a hash of all
/// records within its scope and links to the previous seal in
/// the chain. This provides blockchain-lite tamper evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seal {
    /// Unique seal identifier.
    pub id: SealId,
    /// What records this seal covers.
    pub scope: SealScope,
    /// Hash of all records within the scope.
    pub content_hash: u64,
    /// Hash of the previous seal in the chain.
    pub prev_seal_hash: u64,
    /// Combined hash (content + prev = seal hash).
    pub seal_hash: u64,
    /// Number of records sealed.
    pub record_count: u64,
    /// When the seal was created.
    pub sealed_at: u64,
    /// Who/what created the seal.
    pub sealed_by: String,
}

impl Seal {
    /// Verifies this seal's hash integrity.
    #[must_use]
    pub fn verify(&self) -> bool {
        let recomputed = Self::compute_seal_hash(self.content_hash, self.prev_seal_hash);
        self.seal_hash == recomputed
    }

    /// Computes the combined seal hash.
    fn compute_seal_hash(content_hash: u64, prev_hash: u64) -> u64 {
        let mut h = content_hash;
        h = h.wrapping_mul(41).wrapping_add(prev_hash);
        h = h.wrapping_mul(41).wrapping_add(content_hash.rotate_left(7));
        h
    }
}

impl GroundsTo for Seal {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — permanent
            LexPrimitiva::Persistence,     // π — archival
            LexPrimitiva::Causality,       // → — chain link
            LexPrimitiva::Quantity,        // N — record count
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.90)
    }
}

// ===============================================================
// SEAL CHAIN
// ===============================================================

/// Hash-linked chain of seals (blockchain-lite).
/// Tier: T2-C (∝ + π + →)
///
/// Seals are appended in order and hash-linked. The chain
/// provides tamper evidence: modifying any seal breaks the
/// chain verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealChain {
    /// Ordered seals (append-only).
    seals: Vec<Seal>,
    /// Next seal ID.
    next_id: u64,
    /// Last seal hash (for chain linking).
    last_hash: u64,
}

impl SealChain {
    /// Creates a new empty seal chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seals: Vec::new(),
            next_id: 1,
            last_hash: 0,
        }
    }

    /// Creates a new seal and appends it to the chain.
    /// This is an ∝ operation — the seal cannot be removed.
    pub fn seal(
        &mut self,
        scope: SealScope,
        content_hash: u64,
        record_count: u64,
        sealed_by: &str,
        now: u64,
    ) -> Seal {
        let seal_hash = Seal::compute_seal_hash(content_hash, self.last_hash);

        let seal = Seal {
            id: SealId::new(self.next_id),
            scope,
            content_hash,
            prev_seal_hash: self.last_hash,
            seal_hash,
            record_count,
            sealed_at: now,
            sealed_by: sealed_by.to_string(),
        };

        self.next_id += 1;
        self.last_hash = seal_hash;
        self.seals.push(seal.clone());
        seal
    }

    /// Number of seals in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.seals.len()
    }

    /// Whether the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seals.is_empty()
    }

    /// Returns all seals (read-only).
    #[must_use]
    pub fn seals(&self) -> &[Seal] {
        &self.seals
    }

    /// Gets a seal by ID.
    #[must_use]
    pub fn get(&self, id: SealId) -> Option<&Seal> {
        self.seals.iter().find(|s| s.id == id)
    }

    /// Last seal hash.
    #[must_use]
    pub fn last_hash(&self) -> u64 {
        self.last_hash
    }

    /// Verifies the entire chain's integrity.
    #[must_use]
    pub fn verify(&self) -> TamperVerdict {
        let mut expected_prev = 0u64;

        for (i, seal) in self.seals.iter().enumerate() {
            // Check chain linkage
            if seal.prev_seal_hash != expected_prev {
                return TamperVerdict::BrokenChain { break_index: i };
            }

            // Check seal integrity
            let recomputed = Seal::compute_seal_hash(seal.content_hash, seal.prev_seal_hash);
            if seal.seal_hash != recomputed {
                return TamperVerdict::Tampered {
                    seal_index: i,
                    expected: recomputed,
                    actual: seal.seal_hash,
                };
            }

            expected_prev = seal.seal_hash;
        }

        TamperVerdict::Clean
    }

    /// Total records sealed across all seals.
    #[must_use]
    pub fn total_records(&self) -> u64 {
        self.seals.iter().map(|s| s.record_count).sum()
    }
}

impl Default for SealChain {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for SealChain {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — append-only
            LexPrimitiva::Persistence,     // π — permanent storage
            LexPrimitiva::Causality,       // → — hash chain
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.90)
    }
}

// ===============================================================
// ARCHIVAL PACKAGE
// ===============================================================

/// A sealed records package for long-term archival.
/// Tier: T2-C (π + ∝ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalPackage {
    /// The seal that closes this package.
    pub seal: Seal,
    /// Record content hashes included in this package.
    pub record_hashes: Vec<u64>,
    /// Human-readable manifest.
    pub manifest: String,
    /// Retention period in days.
    pub retention_days: u64,
}

impl ArchivalPackage {
    /// Creates an archival package from a seal.
    #[must_use]
    pub fn new(seal: Seal, record_hashes: Vec<u64>, retention_days: u64) -> Self {
        let manifest = format!(
            "Archival Package: {} | {} records | Retain {} days",
            seal.scope.describe(),
            record_hashes.len(),
            retention_days,
        );
        Self {
            seal,
            record_hashes,
            manifest,
            retention_days,
        }
    }

    /// Verifies that a record hash is in this package.
    #[must_use]
    pub fn contains_record(&self, hash: u64) -> bool {
        self.record_hashes.contains(&hash)
    }

    /// Number of records in the package.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.record_hashes.len()
    }
}

impl GroundsTo for ArchivalPackage {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,     // π — long-term storage
            LexPrimitiva::Irreversibility, // ∝ — sealed records
            LexPrimitiva::Quantity,        // N — retention period
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_seal_chain_create() {
        let mut chain = SealChain::new();
        assert!(chain.is_empty());

        let seal = chain.seal(
            SealScope::quarter(2024, 1),
            0xDEAD,
            100,
            "system",
            1_000_000,
        );

        assert_eq!(chain.len(), 1);
        assert_eq!(seal.record_count, 100);
        assert_eq!(seal.scope, SealScope::Quarter(2024, 1));
    }

    #[test]
    fn test_seal_chain_linking() {
        let mut chain = SealChain::new();

        let s1 = chain.seal(SealScope::quarter(2024, 1), 0xAAA, 50, "sys", 1000);
        let s2 = chain.seal(SealScope::quarter(2024, 2), 0xBBB, 60, "sys", 2000);
        let s3 = chain.seal(SealScope::quarter(2024, 3), 0xCCC, 70, "sys", 3000);

        // Chain linkage
        assert_eq!(s1.prev_seal_hash, 0); // Genesis
        assert_eq!(s2.prev_seal_hash, s1.seal_hash);
        assert_eq!(s3.prev_seal_hash, s2.seal_hash);
    }

    #[test]
    fn test_seal_chain_verification() {
        let mut chain = SealChain::new();

        chain.seal(SealScope::quarter(2024, 1), 0xAAA, 50, "sys", 1000);
        chain.seal(SealScope::quarter(2024, 2), 0xBBB, 60, "sys", 2000);
        chain.seal(SealScope::quarter(2024, 3), 0xCCC, 70, "sys", 3000);

        let verdict = chain.verify();
        assert!(verdict.is_clean());
    }

    #[test]
    fn test_seal_individual_verification() {
        let mut chain = SealChain::new();
        let seal = chain.seal(SealScope::year(2024), 0xFACE, 200, "auditor", 5000);

        assert!(seal.verify());
    }

    #[test]
    fn test_seal_scope_describe() {
        assert_eq!(SealScope::quarter(2024, 1).describe(), "Q1/2024");
        assert_eq!(SealScope::year(2024).describe(), "FY2024");
        assert_eq!(
            SealScope::TimeRange {
                start_epoch: 100,
                end_epoch: 200
            }
            .describe(),
            "epoch:100-200"
        );
        assert_eq!(
            SealScope::RecordRange {
                start_id: 1,
                end_id: 50
            }
            .describe(),
            "records:1-50"
        );
    }

    #[test]
    fn test_seal_scope_quarter_clamping() {
        assert_eq!(SealScope::quarter(2024, 0), SealScope::Quarter(2024, 1));
        assert_eq!(SealScope::quarter(2024, 5), SealScope::Quarter(2024, 4));
    }

    #[test]
    fn test_seal_chain_total_records() {
        let mut chain = SealChain::new();
        chain.seal(SealScope::quarter(2024, 1), 0x1, 100, "sys", 1000);
        chain.seal(SealScope::quarter(2024, 2), 0x2, 150, "sys", 2000);

        assert_eq!(chain.total_records(), 250);
    }

    #[test]
    fn test_archival_package() {
        let mut chain = SealChain::new();
        let seal = chain.seal(SealScope::quarter(2024, 1), 0xABC, 3, "sys", 1000);

        let pkg = ArchivalPackage::new(seal, vec![0xA, 0xB, 0xC], 365 * 7);

        assert_eq!(pkg.record_count(), 3);
        assert!(pkg.contains_record(0xA));
        assert!(!pkg.contains_record(0xD));
        assert!(pkg.manifest.contains("Q1/2024"));
        assert_eq!(pkg.retention_days, 365 * 7);
    }

    #[test]
    fn test_tamper_verdict() {
        assert!(TamperVerdict::Clean.is_clean());
        assert!(
            !TamperVerdict::Tampered {
                seal_index: 0,
                expected: 1,
                actual: 2,
            }
            .is_clean()
        );
        assert!(!TamperVerdict::BrokenChain { break_index: 0 }.is_clean());
    }

    #[test]
    fn test_seal_grounding() {
        let comp = Seal::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_seal_chain_grounding() {
        let comp = SealChain::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_archival_grounding() {
        let comp = ArchivalPackage::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_seal_id_display() {
        let id = SealId::new(42);
        assert_eq!(format!("{id}"), "SEAL-0000002A");
    }
}
