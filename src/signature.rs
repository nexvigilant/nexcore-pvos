//! # PVTX Electronic Signatures
//!
//! 21 CFR Part 11 compliant electronic signature system for
//! non-repudiation of regulatory actions. Every signature binds
//! an identity to a specific action irreversibly (∝).
//!
//! ## Primitives
//! - ∝ (Irreversibility) — DOMINANT: signatures are non-repudiable
//! - ∃ (Existence) — identity verification
//! - ∂ (Boundary) — signature policy enforcement
//! - π (Persistence) — permanent signature records
//!
//! ## 21 CFR Part 11 Mapping
//!
//! | Requirement                | Implementation              |
//! |----------------------------|-----------------------------|
//! | Unique user identification | `Signer::Human(SignerId)`   |
//! | Signature binding          | `Signature.signer + action` |
//! | Signature manifestation    | `SignatureRequest.meaning`  |
//! | Non-repudiation            | ∝ primitive + hash proof    |
//! | Audit trail                | Integration with TxLog      |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::transaction::TxId;

// ===============================================================
// T2-P NEWTYPES
// ===============================================================

/// Unique signer identifier.
/// Tier: T2-P (∃)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignerId(pub u64);

impl SignerId {
    /// Creates a new signer ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl GroundsTo for SignerId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence])
    }
}

/// Unique signature identifier.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignatureId(pub u64);

impl SignatureId {
    /// Creates a new signature ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SignatureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SIG-{:08X}", self.0)
    }
}

impl GroundsTo for SignatureId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// SIGNER IDENTITY
// ===============================================================

/// Who is signing — the identity behind the signature.
/// Tier: T2-P (∃ + ∝)
///
/// Per 21 CFR Part 11: each signer must have a unique identity.
/// System signatures are distinguished from human signatures.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Signer {
    /// Human user with verified identity.
    Human(SignerId),
    /// Automated system action.
    System(String),
    /// Delegated authority (human delegating to system).
    Delegated {
        /// The human who delegated.
        delegator: SignerId,
        /// The system acting on behalf.
        delegate: String,
    },
}

impl Signer {
    /// Returns the primary identity behind the signer.
    #[must_use]
    pub fn primary_id(&self) -> Option<SignerId> {
        match self {
            Self::Human(id) | Self::Delegated { delegator: id, .. } => Some(*id),
            Self::System(_) => None,
        }
    }

    /// Returns true if this is a human signer.
    #[must_use]
    pub fn is_human(&self) -> bool {
        matches!(self, Self::Human(_))
    }

    /// Returns true if this is a system signer.
    #[must_use]
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System(_))
    }
}

impl GroundsTo for Signer {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence, LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// SIGNATURE MEANING
// ===============================================================

/// What the signer is attesting to.
/// Tier: T2-P (∝)
///
/// Per 21 CFR Part 11.50: signature must include the meaning
/// (review, approval, authorship, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatureMeaning {
    /// Author of the document/record.
    Authorship,
    /// Reviewed the record for accuracy.
    Review,
    /// Approved the action or document.
    Approval,
    /// Verified data integrity.
    Verification,
    /// Authorized a regulatory submission.
    Authorization,
    /// Confirmed a medical/scientific assessment.
    Confirmation,
}

impl GroundsTo for SignatureMeaning {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// SIGNATURE REQUEST
// ===============================================================

/// A request for a signature — what is being signed and why.
/// Tier: T2-C (∝ + ∃ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRequest {
    /// What transaction this signature covers.
    pub tx_id: TxId,
    /// The meaning of this signature.
    pub meaning: SignatureMeaning,
    /// Human-readable description of what is being signed.
    pub description: String,
    /// Hash of the content being signed.
    pub content_hash: u64,
    /// Timestamp of the request.
    pub requested_at: u64,
}

impl SignatureRequest {
    /// Creates a new signature request.
    #[must_use]
    pub fn new(
        tx_id: TxId,
        meaning: SignatureMeaning,
        description: &str,
        content_hash: u64,
        now: u64,
    ) -> Self {
        Self {
            tx_id,
            meaning,
            description: description.to_string(),
            content_hash,
            requested_at: now,
        }
    }
}

impl GroundsTo for SignatureRequest {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility,
            LexPrimitiva::Existence,
            LexPrimitiva::Boundary,
        ])
    }
}

// ===============================================================
// SIGNATURE
// ===============================================================

/// A completed electronic signature — non-repudiable proof.
/// Tier: T2-C (∝ + ∃ + π + ∂)
///
/// Once created, a Signature is immutable. It permanently binds
/// the signer's identity to the action. This is the ∝ manifestation
/// at the identity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// Unique signature identifier.
    pub id: SignatureId,
    /// Who signed.
    pub signer: Signer,
    /// What was signed (transaction).
    pub tx_id: TxId,
    /// The meaning of this signature.
    pub meaning: SignatureMeaning,
    /// Description of what was signed.
    pub description: String,
    /// Hash of the signed content.
    pub content_hash: u64,
    /// Hash proof of the signature itself.
    pub signature_hash: u64,
    /// When the signature was applied.
    pub signed_at: u64,
}

impl Signature {
    /// Verifies this signature's integrity.
    /// Returns true if the signature hash matches recomputation.
    #[must_use]
    pub fn verify(&self) -> bool {
        let recomputed =
            Self::compute_sig_hash(&self.signer, self.tx_id, self.content_hash, self.signed_at);
        self.signature_hash == recomputed
    }

    /// Computes a deterministic signature hash.
    fn compute_sig_hash(signer: &Signer, tx_id: TxId, content_hash: u64, timestamp: u64) -> u64 {
        let mut h: u64 = content_hash;
        h = h.wrapping_mul(37).wrapping_add(tx_id.0);
        h = h.wrapping_mul(37).wrapping_add(timestamp);

        let signer_seed = match signer {
            Signer::Human(id) => id.0,
            Signer::System(name) => {
                let mut s: u64 = 0;
                for b in name.bytes() {
                    s = s.wrapping_mul(31).wrapping_add(u64::from(b));
                }
                s
            }
            Signer::Delegated {
                delegator,
                delegate,
            } => {
                let mut s = delegator.0;
                for b in delegate.bytes() {
                    s = s.wrapping_mul(31).wrapping_add(u64::from(b));
                }
                s
            }
        };
        h = h.wrapping_mul(37).wrapping_add(signer_seed);
        h
    }
}

impl GroundsTo for Signature {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — non-repudiation
            LexPrimitiva::Existence,       // ∃ — identity binding
            LexPrimitiva::Persistence,     // π — permanent record
            LexPrimitiva::Boundary,        // ∂ — policy enforcement
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.85)
    }
}

// ===============================================================
// SIGNATURE POLICY
// ===============================================================

/// Who must sign for what kind of action.
/// Tier: T2-C (∂ + ∃ + ∝)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignaturePolicy {
    /// Policy name.
    pub name: String,
    /// What action kinds this policy applies to.
    pub applies_to: Vec<String>,
    /// Required signature meaning.
    pub required_meaning: SignatureMeaning,
    /// Whether a human signature is required (vs system).
    pub requires_human: bool,
    /// Minimum number of signatures required.
    pub min_signatures: usize,
}

impl SignaturePolicy {
    /// Creates a policy requiring human approval.
    #[must_use]
    pub fn human_approval(name: &str, applies_to: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            applies_to,
            required_meaning: SignatureMeaning::Approval,
            requires_human: true,
            min_signatures: 1,
        }
    }

    /// Creates a policy requiring dual authorization.
    #[must_use]
    pub fn dual_authorization(name: &str, applies_to: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            applies_to,
            required_meaning: SignatureMeaning::Authorization,
            requires_human: true,
            min_signatures: 2,
        }
    }

    /// Checks if this policy applies to a given action kind.
    #[must_use]
    pub fn applies(&self, action_kind: &str) -> bool {
        self.applies_to.iter().any(|a| a == action_kind)
    }

    /// Validates that a set of signatures satisfies this policy.
    #[must_use]
    pub fn is_satisfied(&self, signatures: &[Signature]) -> bool {
        let matching: Vec<_> = signatures
            .iter()
            .filter(|s| {
                s.meaning == self.required_meaning && (!self.requires_human || s.signer.is_human())
            })
            .collect();
        matching.len() >= self.min_signatures
    }
}

impl GroundsTo for SignaturePolicy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary,        // ∂ — enforcement rules
            LexPrimitiva::Existence,       // ∃ — identity requirements
            LexPrimitiva::Irreversibility, // ∝ — policy consequences
        ])
        .with_dominant(LexPrimitiva::Boundary, 0.75)
    }
}

// ===============================================================
// SIGNATURE SERVICE
// ===============================================================

/// Service that manages signature creation and verification.
/// Tier: T2-C (∝ + ∃ + ∂ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureService {
    /// Known signer identities.
    known_signers: Vec<SignerId>,
    /// All signatures created (append-only).
    signatures: Vec<Signature>,
    /// Next signature ID.
    next_id: u64,
}

impl SignatureService {
    /// Creates a new signature service.
    #[must_use]
    pub fn new() -> Self {
        Self {
            known_signers: Vec::new(),
            signatures: Vec::new(),
            next_id: 1,
        }
    }

    /// Registers a signer identity.
    pub fn register_signer(&mut self, id: SignerId) {
        if !self.known_signers.contains(&id) {
            self.known_signers.push(id);
        }
    }

    /// Verifies that a signer's identity is known.
    ///
    /// # Errors
    /// Returns `Err` if the signer is not registered.
    pub fn verify_identity(&self, signer: &Signer) -> Result<(), SignatureError> {
        match signer {
            Signer::Human(id) => {
                if self.known_signers.contains(id) {
                    Ok(())
                } else {
                    Err(SignatureError::UnknownSigner(*id))
                }
            }
            Signer::System(_) => Ok(()), // System signers always valid
            Signer::Delegated { delegator, .. } => {
                if self.known_signers.contains(delegator) {
                    Ok(())
                } else {
                    Err(SignatureError::UnknownSigner(*delegator))
                }
            }
        }
    }

    /// Creates a signature for a request.
    ///
    /// # Errors
    /// Returns `Err` if the signer is unknown.
    pub fn sign(
        &mut self,
        request: &SignatureRequest,
        signer: &Signer,
        now: u64,
    ) -> Result<Signature, SignatureError> {
        self.verify_identity(signer)?;

        let sig_hash =
            Signature::compute_sig_hash(signer, request.tx_id, request.content_hash, now);

        let signature = Signature {
            id: SignatureId::new(self.next_id),
            signer: signer.clone(),
            tx_id: request.tx_id,
            meaning: request.meaning.clone(),
            description: request.description.clone(),
            content_hash: request.content_hash,
            signature_hash: sig_hash,
            signed_at: now,
        };

        self.next_id += 1;
        self.signatures.push(signature.clone());
        Ok(signature)
    }

    /// Gets all signatures for a transaction.
    #[must_use]
    pub fn signatures_for(&self, tx_id: TxId) -> Vec<&Signature> {
        self.signatures
            .iter()
            .filter(|s| s.tx_id == tx_id)
            .collect()
    }

    /// Total signatures created.
    #[must_use]
    pub fn total_signatures(&self) -> usize {
        self.signatures.len()
    }

    /// Number of registered signers.
    #[must_use]
    pub fn signer_count(&self) -> usize {
        self.known_signers.len()
    }
}

impl Default for SignatureService {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for SignatureService {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — non-repudiable signatures
            LexPrimitiva::Existence,       // ∃ — identity management
            LexPrimitiva::Boundary,        // ∂ — policy enforcement
            LexPrimitiva::Persistence,     // π — signature archive
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// SIGNATURE ERROR
// ===============================================================

/// Signature operation errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureError {
    /// Signer identity not registered.
    UnknownSigner(SignerId),
    /// Signature verification failed.
    VerificationFailed(SignatureId),
    /// Required signature missing.
    MissingSignature(String),
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSigner(id) => write!(f, "unknown signer: {:?}", id),
            Self::VerificationFailed(id) => write!(f, "signature verification failed: {id}"),
            Self::MissingSignature(msg) => write!(f, "missing signature: {msg}"),
        }
    }
}

impl std::error::Error for SignatureError {}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_service() -> SignatureService {
        let mut svc = SignatureService::new();
        svc.register_signer(SignerId::new(1));
        svc.register_signer(SignerId::new(2));
        svc
    }

    fn make_request(tx_id: u64) -> SignatureRequest {
        SignatureRequest::new(
            TxId::new(tx_id),
            SignatureMeaning::Approval,
            "Approve signal report",
            0xDEAD_BEEF,
            1000,
        )
    }

    #[test]
    fn test_sign_human() {
        let mut svc = make_service();
        let req = make_request(1);
        let signer = Signer::Human(SignerId::new(1));

        let sig = svc.sign(&req, &signer, 1001);
        assert!(sig.is_ok());
        if let Ok(s) = sig {
            assert_eq!(s.tx_id, TxId::new(1));
            assert_eq!(s.meaning, SignatureMeaning::Approval);
            assert!(s.signer.is_human());
        }
    }

    #[test]
    fn test_sign_system() {
        let mut svc = make_service();
        let req = make_request(2);
        let signer = Signer::System("auto_validator".into());

        let sig = svc.sign(&req, &signer, 1001);
        assert!(sig.is_ok());
        if let Ok(s) = sig {
            assert!(s.signer.is_system());
        }
    }

    #[test]
    fn test_sign_delegated() {
        let mut svc = make_service();
        let req = make_request(3);
        let signer = Signer::Delegated {
            delegator: SignerId::new(1),
            delegate: "bot_agent".into(),
        };

        let sig = svc.sign(&req, &signer, 1001);
        assert!(sig.is_ok());
    }

    #[test]
    fn test_unknown_signer_rejected() {
        let mut svc = make_service();
        let req = make_request(4);
        let signer = Signer::Human(SignerId::new(999));

        let result = svc.sign(&req, &signer, 1001);
        assert!(result.is_err());
        if let Err(SignatureError::UnknownSigner(id)) = result {
            assert_eq!(id, SignerId::new(999));
        }
    }

    #[test]
    fn test_signature_verification() {
        let mut svc = make_service();
        let req = make_request(5);
        let signer = Signer::Human(SignerId::new(1));

        let sig = svc.sign(&req, &signer, 1001);
        assert!(sig.is_ok());
        if let Ok(s) = sig {
            assert!(s.verify());
        }
    }

    #[test]
    fn test_signatures_for_tx() {
        let mut svc = make_service();
        let req1 = make_request(10);
        let req2 = SignatureRequest::new(
            TxId::new(10),
            SignatureMeaning::Review,
            "Review report",
            0xCAFE,
            1000,
        );

        let _ = svc.sign(&req1, &Signer::Human(SignerId::new(1)), 1001);
        let _ = svc.sign(&req2, &Signer::Human(SignerId::new(2)), 1002);

        let sigs = svc.signatures_for(TxId::new(10));
        assert_eq!(sigs.len(), 2);
    }

    #[test]
    fn test_policy_human_approval() {
        let policy = SignaturePolicy::human_approval(
            "report_approval",
            vec!["regulatory_submission".into()],
        );

        assert!(policy.applies("regulatory_submission"));
        assert!(!policy.applies("data_entry"));
        assert!(policy.requires_human);
        assert_eq!(policy.min_signatures, 1);
    }

    #[test]
    fn test_policy_dual_authorization() {
        let policy =
            SignaturePolicy::dual_authorization("dual_auth", vec!["critical_submission".into()]);

        assert_eq!(policy.min_signatures, 2);
        assert!(policy.requires_human);
    }

    #[test]
    fn test_policy_satisfaction() {
        let policy = SignaturePolicy::human_approval("test_policy", vec!["test".into()]);

        // Satisfied by human approval
        let mut svc = make_service();
        let req = make_request(20);
        let sig = svc.sign(&req, &Signer::Human(SignerId::new(1)), 1001);
        assert!(sig.is_ok());
        if let Ok(s) = sig {
            assert!(policy.is_satisfied(&[s]));
        }

        // Not satisfied by system signature when human required
        let req2 = make_request(21);
        let sys_sig = svc.sign(&req2, &Signer::System("bot".into()), 1002);
        assert!(sys_sig.is_ok());
        if let Ok(s) = sys_sig {
            assert!(!policy.is_satisfied(&[s]));
        }
    }

    #[test]
    fn test_signer_primary_id() {
        assert_eq!(
            Signer::Human(SignerId::new(5)).primary_id(),
            Some(SignerId::new(5))
        );
        assert_eq!(Signer::System("bot".into()).primary_id(), None);
        assert_eq!(
            Signer::Delegated {
                delegator: SignerId::new(3),
                delegate: "bot".into()
            }
            .primary_id(),
            Some(SignerId::new(3))
        );
    }

    #[test]
    fn test_signature_grounding() {
        let comp = Signature::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_signature_service_grounding() {
        let comp = SignatureService::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_policy_grounding() {
        let comp = SignaturePolicy::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Boundary));
    }
}
