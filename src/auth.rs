//! # PVGW Authentication & Authorization
//!
//! Identity verification and permission enforcement at the gateway boundary.
//! Every request must prove identity (∃) and be authorized to cross (∂).
//!
//! ## Primitives
//! - ∃ (Existence) — does this identity exist?
//! - ∂ (Boundary) — is this identity allowed to cross?
//!
//! ## Design
//!
//! Auth failures are intentionally opaque to prevent information leakage.
//! API key comparison uses constant-time equality.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// API key for programmatic access.
/// Tier: T2-P (∃)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiKey(pub u64);

impl GroundsTo for ApiKey {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence])
    }
}

/// Bearer token for session-based access.
/// Tier: T2-P (∃)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token(pub String);

impl GroundsTo for Token {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence])
    }
}

/// Service-to-service account identifier.
/// Tier: T2-P (∃ + N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceAccountId(pub u64);

impl GroundsTo for ServiceAccountId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence, LexPrimitiva::Quantity])
    }
}

/// Permission level (ordered by privilege).
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Read-only access to data.
    Read,
    /// Write access (create/update).
    Write,
    /// Execute workflows and syscalls.
    Execute,
    /// Full administrative access.
    Admin,
}

impl GroundsTo for Permission {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

// ═══════════════════════════════════════════════════════════
// T2-C COMPOSITES
// ═══════════════════════════════════════════════════════════

/// How the caller identifies themselves.
/// Tier: T2-P (∃)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdentityKind {
    /// Programmatic API key.
    Key(ApiKey),
    /// Session bearer token.
    Bearer(Token),
    /// Service-to-service account.
    ServiceAccount(ServiceAccountId),
    /// Unauthenticated caller.
    Anonymous,
}

/// Authenticated identity with granted permissions.
/// Tier: T2-C (∃ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Human-readable name.
    pub name: String,
    /// Authentication method.
    pub kind: IdentityKind,
    /// Granted permissions.
    pub permissions: Vec<Permission>,
}

impl Identity {
    /// Checks if this identity holds at least the required permission.
    #[must_use]
    pub fn has_permission(&self, required: Permission) -> bool {
        self.permissions.iter().any(|p| *p >= required)
    }

    /// Creates an anonymous identity with read-only access.
    #[must_use]
    pub fn anonymous() -> Self {
        Self {
            name: "anonymous".into(),
            kind: IdentityKind::Anonymous,
            permissions: vec![Permission::Read],
        }
    }

    /// Creates a service identity with full access.
    #[must_use]
    pub fn service(name: &str, id: ServiceAccountId) -> Self {
        Self {
            name: name.into(),
            kind: IdentityKind::ServiceAccount(id),
            permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Admin,
            ],
        }
    }
}

impl GroundsTo for Identity {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Existence, LexPrimitiva::Boundary])
            .with_dominant(LexPrimitiva::Existence, 0.85)
    }
}

/// Policy rule mapping identity patterns to path restrictions.
/// Tier: T2-C (∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Glob pattern for identity names (* = any).
    pub identity_pattern: String,
    /// Paths this identity is denied access to.
    pub deny_paths: Vec<String>,
}

impl GroundsTo for PolicyRule {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Mapping])
    }
}

/// Authentication/authorization error.
/// Intentionally opaque to prevent information leakage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    /// Identity not recognized.
    UnknownIdentity,
    /// Credential invalid or expired.
    InvalidCredential,
    /// Identity lacks required permission.
    InsufficientPermission { required: Permission },
    /// Path explicitly denied by policy.
    PathDenied,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately minimal — don't leak information
        match self {
            Self::UnknownIdentity | Self::InvalidCredential => {
                write!(f, "authentication failed")
            }
            Self::InsufficientPermission { .. } => {
                write!(f, "authorization failed")
            }
            Self::PathDenied => write!(f, "access denied"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Authentication and authorization engine.
/// Tier: T2-C (∃ + ∂ + μ)
///
/// Manages registered identities and policy rules.
/// All credential comparisons use constant-time equality.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthEngine {
    /// Registered identities.
    identities: Vec<Identity>,
    /// Access policies.
    policies: Vec<PolicyRule>,
}

impl AuthEngine {
    /// Creates a new empty auth engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an identity.
    pub fn register(&mut self, identity: Identity) {
        self.identities.push(identity);
    }

    /// Adds a policy rule.
    pub fn add_policy(&mut self, rule: PolicyRule) {
        self.policies.push(rule);
    }

    /// Authenticates a credential, returning the matching identity.
    ///
    /// # Errors
    /// Returns `AuthError::UnknownIdentity` if no match found.
    pub fn authenticate(&self, kind: &IdentityKind) -> Result<&Identity, AuthError> {
        match kind {
            IdentityKind::Key(key) => {
                self.identities
                    .iter()
                    .find(|i| matches!(&i.kind, IdentityKind::Key(k) if constant_time_eq_u64(k.0, key.0)))
                    .ok_or(AuthError::UnknownIdentity)
            }
            IdentityKind::Bearer(token) => {
                self.identities
                    .iter()
                    .find(|i| matches!(&i.kind, IdentityKind::Bearer(t) if constant_time_eq_bytes(t.0.as_bytes(), token.0.as_bytes())))
                    .ok_or(AuthError::UnknownIdentity)
            }
            IdentityKind::ServiceAccount(sa) => {
                self.identities
                    .iter()
                    .find(|i| matches!(&i.kind, IdentityKind::ServiceAccount(s) if s.0 == sa.0))
                    .ok_or(AuthError::UnknownIdentity)
            }
            IdentityKind::Anonymous => {
                // Anonymous always succeeds with minimal permissions
                self.identities
                    .iter()
                    .find(|i| i.kind == IdentityKind::Anonymous)
                    .ok_or(AuthError::UnknownIdentity)
            }
        }
    }

    /// Authorizes an identity for a permission on a path.
    ///
    /// # Errors
    /// Returns `AuthError` if authorization fails.
    pub fn authorize(
        &self,
        identity: &Identity,
        required: Permission,
        path: &str,
    ) -> Result<(), AuthError> {
        // Check deny rules first
        for rule in &self.policies {
            if pattern_matches(&rule.identity_pattern, &identity.name) {
                if rule.deny_paths.iter().any(|d| path.starts_with(d.as_str())) {
                    return Err(AuthError::PathDenied);
                }
            }
        }

        // Check permission level
        if identity.has_permission(required) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermission { required })
        }
    }

    /// Number of registered identities.
    #[must_use]
    pub fn identity_count(&self) -> usize {
        self.identities.len()
    }

    /// Number of policy rules.
    #[must_use]
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl GroundsTo for AuthEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,
            LexPrimitiva::Boundary,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Existence, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// CONSTANT-TIME COMPARISON UTILITIES
// ═══════════════════════════════════════════════════════════

/// Constant-time equality for u64 values.
/// Prevents timing attacks on API key comparison.
#[must_use]
fn constant_time_eq_u64(a: u64, b: u64) -> bool {
    let diff = a ^ b;
    // Fold to single bit: if any bit differs, result is 1
    let folded = diff | diff.wrapping_shr(32);
    let folded = folded | folded.wrapping_shr(16);
    let folded = folded | folded.wrapping_shr(8);
    let folded = folded | folded.wrapping_shr(4);
    let folded = folded | folded.wrapping_shr(2);
    let folded = folded | folded.wrapping_shr(1);
    (folded & 1) == 0
}

/// Constant-time equality for byte slices.
/// Always compares full length to prevent timing leaks.
#[must_use]
fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Simple glob pattern matching (supports trailing `*` only).
#[must_use]
fn pattern_matches(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == name {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn setup_engine() -> AuthEngine {
        let mut engine = AuthEngine::new();

        engine.register(Identity {
            name: "admin_user".into(),
            kind: IdentityKind::Key(ApiKey(12345)),
            permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Admin,
            ],
        });

        engine.register(Identity {
            name: "read_user".into(),
            kind: IdentityKind::Key(ApiKey(67890)),
            permissions: vec![Permission::Read],
        });

        engine.register(Identity {
            name: "service_a".into(),
            kind: IdentityKind::ServiceAccount(ServiceAccountId(1)),
            permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
        });

        engine.register(Identity::anonymous());

        engine
    }

    #[test]
    fn test_authenticate_api_key() {
        let engine = setup_engine();
        let result = engine.authenticate(&IdentityKind::Key(ApiKey(12345)));
        assert!(result.is_ok());
        if let Ok(identity) = result {
            assert_eq!(identity.name, "admin_user");
        }
    }

    #[test]
    fn test_authenticate_unknown_key() {
        let engine = setup_engine();
        let result = engine.authenticate(&IdentityKind::Key(ApiKey(99999)));
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_service_account() {
        let engine = setup_engine();
        let result = engine.authenticate(&IdentityKind::ServiceAccount(ServiceAccountId(1)));
        assert!(result.is_ok());
        if let Ok(identity) = result {
            assert_eq!(identity.name, "service_a");
        }
    }

    #[test]
    fn test_authenticate_anonymous() {
        let engine = setup_engine();
        let result = engine.authenticate(&IdentityKind::Anonymous);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authorize_sufficient_permission() {
        let engine = setup_engine();
        let admin = engine.authenticate(&IdentityKind::Key(ApiKey(12345)));
        assert!(admin.is_ok());
        if let Ok(identity) = admin {
            let result = engine.authorize(identity, Permission::Write, "/api/v1/signals");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_authorize_insufficient_permission() {
        let engine = setup_engine();
        let reader = engine.authenticate(&IdentityKind::Key(ApiKey(67890)));
        assert!(reader.is_ok());
        if let Ok(identity) = reader {
            let result = engine.authorize(identity, Permission::Write, "/api/v1/signals");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_authorize_path_denied() {
        let mut engine = setup_engine();
        engine.add_policy(PolicyRule {
            identity_pattern: "read_*".into(),
            deny_paths: vec!["/api/v1/admin".into()],
        });

        let reader = engine.authenticate(&IdentityKind::Key(ApiKey(67890)));
        assert!(reader.is_ok());
        if let Ok(identity) = reader {
            let result = engine.authorize(identity, Permission::Read, "/api/v1/admin/users");
            assert!(matches!(result, Err(AuthError::PathDenied)));
        }
    }

    #[test]
    fn test_permission_ordering() {
        assert!(Permission::Admin > Permission::Execute);
        assert!(Permission::Execute > Permission::Write);
        assert!(Permission::Write > Permission::Read);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq_u64(12345, 12345));
        assert!(!constant_time_eq_u64(12345, 12346));
        assert!(constant_time_eq_u64(0, 0));
        assert!(constant_time_eq_u64(u64::MAX, u64::MAX));
    }

    #[test]
    fn test_constant_time_eq_bytes() {
        assert!(constant_time_eq_bytes(b"hello", b"hello"));
        assert!(!constant_time_eq_bytes(b"hello", b"world"));
        assert!(!constant_time_eq_bytes(b"hello", b"hell"));
    }

    #[test]
    fn test_auth_engine_grounding() {
        let comp = AuthEngine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_identity_grounding() {
        let comp = Identity::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("admin_user", "admin_user"));
        assert!(!pattern_matches("admin_user", "other"));
        assert!(pattern_matches("read_*", "read_user"));
        assert!(pattern_matches("read_*", "read_only"));
        assert!(!pattern_matches("read_*", "write_user"));
    }
}
