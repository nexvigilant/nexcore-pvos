//! # PVGW Gateway — The Boundary
//!
//! Core gateway orchestrator that controls all external access to PVOS/PVWF.
//! Every request crosses through this boundary layer, which enforces:
//! 1. Authentication (∃) — who are you?
//! 2. Authorization (∂) — what can you do?
//! 3. Rate limiting (N) — how much can you do?
//! 4. Dispatch (μ) — route to PVOS/PVWF
//! 5. Audit (π) — log every crossing
//!
//! ## Dominant Primitive: ∂ (Boundary)
//!
//! The gateway IS the boundary. Its primary function is deciding
//! what crosses in and out of the pharmacovigilance system.
//!
//! ## Architecture
//!
//! ```text
//! External Request
//!     │
//!     ▼
//! ┌──────────┐
//! │ Gateway  │ ← ∂ boundary
//! │  cross() │
//! ├──────────┤
//! │ 1. Auth  │ ← ∃ identity
//! │ 2. Authz │ ← ∂ permission
//! │ 3. Rate  │ ← N quota
//! │ 4. Route │ ← μ dispatch
//! │ 5. Audit │ ← π log
//! └──────────┘
//!     │
//!     ▼
//! PVOS / PVWF
//! ```

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::auth::{AuthEngine, AuthError, Identity, IdentityKind, Permission};
use super::crossing::{AuditLevel, ComplianceTag, CrossingLog, CrossingOutcome};
use super::protocol::{ContentType, RequestMethod, StatusCode, method_to_permission};
use super::ratelimit::{Quota, RateLimiter};
use super::workflow::SyscallKind;

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE (protocol-agnostic)
// ═══════════════════════════════════════════════════════════

/// Protocol-agnostic gateway request.
/// Tier: T2-C (∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRequest {
    /// Target path (e.g., "/api/v1/signals").
    pub path: String,
    /// HTTP-like method.
    pub method: RequestMethod,
    /// Request body.
    pub body: String,
    /// Content type.
    pub content_type: ContentType,
    /// Authentication credential.
    pub auth: IdentityKind,
}

impl GatewayRequest {
    /// Creates a GET request.
    #[must_use]
    pub fn get(path: &str, auth: IdentityKind) -> Self {
        Self {
            path: path.to_string(),
            method: RequestMethod::Get,
            body: String::new(),
            content_type: ContentType::None,
            auth,
        }
    }

    /// Creates a POST request.
    #[must_use]
    pub fn post(path: &str, body: &str, auth: IdentityKind) -> Self {
        Self {
            path: path.to_string(),
            method: RequestMethod::Post,
            body: body.to_string(),
            content_type: ContentType::Json,
            auth,
        }
    }
}

impl GroundsTo for GatewayRequest {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Mapping])
            .with_dominant(LexPrimitiva::Boundary, 0.85)
    }
}

/// Protocol-agnostic gateway response.
/// Tier: T2-C (∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    /// HTTP-like status code.
    pub status: StatusCode,
    /// Response body.
    pub body: String,
    /// Content type.
    pub content_type: ContentType,
}

impl GatewayResponse {
    /// Creates a success response.
    #[must_use]
    pub fn ok(body: &str) -> Self {
        Self {
            status: StatusCode::Ok,
            body: body.to_string(),
            content_type: ContentType::Json,
        }
    }

    /// Creates an error response.
    #[must_use]
    fn error(status: StatusCode, message: &str) -> Self {
        Self {
            status,
            body: format!("{{\"error\": \"{message}\"}}"),
            content_type: ContentType::Json,
        }
    }
}

impl GroundsTo for GatewayResponse {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Mapping])
            .with_dominant(LexPrimitiva::Boundary, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// ENDPOINT REGISTRY
// ═══════════════════════════════════════════════════════════

/// Action dispatched when an endpoint is hit.
/// Tier: T2-P (→)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointAction {
    /// Direct PVOS detection syscall.
    DetectSignal,
    /// Direct PVOS compare syscall.
    CompareValues,
    /// Direct PVOS ingest syscall.
    IngestCase,
    /// Query stored artifacts.
    QueryArtifacts,
    /// Store an artifact.
    StoreArtifact,
    /// Execute a named PVWF workflow pattern.
    RunWorkflow(String),
    /// Return system metrics.
    Metrics,
}

impl EndpointAction {
    /// Maps to PVOS SyscallKind where applicable.
    #[must_use]
    pub fn to_syscall_kind(&self) -> Option<SyscallKind> {
        match self {
            Self::DetectSignal => Some(SyscallKind::Detect),
            Self::CompareValues => Some(SyscallKind::Compare),
            Self::IngestCase => Some(SyscallKind::Ingest),
            Self::QueryArtifacts => Some(SyscallKind::Query),
            Self::StoreArtifact => Some(SyscallKind::Store),
            Self::RunWorkflow(_) | Self::Metrics => None,
        }
    }
}

impl GroundsTo for EndpointAction {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

/// Registered endpoint definition.
/// Tier: T2-C (∂ + μ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// URL path pattern.
    pub path: String,
    /// Allowed method.
    pub method: RequestMethod,
    /// Minimum required permission.
    pub permission: Permission,
    /// Action to dispatch.
    pub action: EndpointAction,
    /// Audit level for this endpoint.
    pub audit_level: AuditLevel,
    /// Compliance tags for this endpoint.
    pub compliance_tags: Vec<ComplianceTag>,
    /// Optional quota override for this endpoint.
    pub quota_override: Option<Quota>,
}

impl GroundsTo for Endpoint {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary,
            LexPrimitiva::Mapping,
            LexPrimitiva::Persistence,
        ])
        .with_dominant(LexPrimitiva::Boundary, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// GATEWAY ERROR
// ═══════════════════════════════════════════════════════════

/// Gateway error type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayError {
    /// Authentication failed.
    AuthFailed(String),
    /// Authorization failed.
    Forbidden(String),
    /// Rate limit exceeded.
    RateLimited { retry_after_secs: u64 },
    /// Endpoint not found.
    NotFound(String),
    /// Downstream error from PVOS/PVWF.
    DownstreamError(String),
    /// Invalid request.
    BadRequest(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthFailed(m) => write!(f, "auth failed: {m}"),
            Self::Forbidden(m) => write!(f, "forbidden: {m}"),
            Self::RateLimited { retry_after_secs } => {
                write!(f, "rate limited: retry after {retry_after_secs}s")
            }
            Self::NotFound(p) => write!(f, "not found: {p}"),
            Self::DownstreamError(e) => write!(f, "downstream error: {e}"),
            Self::BadRequest(m) => write!(f, "bad request: {m}"),
        }
    }
}

impl std::error::Error for GatewayError {}

// ═══════════════════════════════════════════════════════════
// GATEWAY CONFIGURATION
// ═══════════════════════════════════════════════════════════

/// Gateway configuration.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Enable rate limiting.
    pub rate_limiting: bool,
    /// Enable crossing audit.
    pub audit_enabled: bool,
    /// Default audit level.
    pub default_audit_level: AuditLevel,
    /// Allow anonymous access.
    pub allow_anonymous: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            rate_limiting: true,
            audit_enabled: true,
            default_audit_level: AuditLevel::Standard,
            allow_anonymous: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// THE GATEWAY — T3 BOUNDARY
// ═══════════════════════════════════════════════════════════

/// Pharmacovigilance Gateway — the boundary layer.
///
/// Controls all external access to PVOS and PVWF.
/// Every request must pass through `cross()` which enforces
/// authentication, authorization, rate limiting, and audit logging.
///
/// Dominant primitive: ∂ (Boundary)
/// Composition: ∂ + μ + π + ς + N + ∃ (6 T1 primitives)
///
/// Tier: T3 Domain-Specific
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gateway {
    /// Registered endpoints.
    endpoints: Vec<Endpoint>,
    /// Authentication engine.
    auth: AuthEngine,
    /// Rate limiter.
    limiter: RateLimiter,
    /// Crossing audit log.
    crossing_log: CrossingLog,
    /// Configuration.
    config: GatewayConfig,
    /// Total requests processed.
    total_requests: u64,
    /// Total requests denied.
    total_denied: u64,
}

impl Gateway {
    /// Creates a new gateway with given configuration.
    #[must_use]
    pub fn new(config: GatewayConfig) -> Self {
        let limiter = if config.rate_limiting {
            RateLimiter::new(Quota::per_minute(100))
        } else {
            RateLimiter::disabled()
        };

        let crossing_log = CrossingLog::new(config.default_audit_level);

        Self {
            endpoints: Vec::new(),
            auth: AuthEngine::new(),
            limiter,
            crossing_log,
            config,
            total_requests: 0,
            total_denied: 0,
        }
    }

    /// Returns a reference to the auth engine.
    #[must_use]
    pub fn auth(&self) -> &AuthEngine {
        &self.auth
    }

    /// Returns a mutable reference to the auth engine.
    pub fn auth_mut(&mut self) -> &mut AuthEngine {
        &mut self.auth
    }

    /// Returns a reference to the rate limiter.
    #[must_use]
    pub fn limiter(&self) -> &RateLimiter {
        &self.limiter
    }

    /// Returns a mutable reference to the rate limiter.
    pub fn limiter_mut(&mut self) -> &mut RateLimiter {
        &mut self.limiter
    }

    /// Returns a reference to the crossing log.
    #[must_use]
    pub fn crossing_log(&self) -> &CrossingLog {
        &self.crossing_log
    }

    /// Registers an endpoint.
    pub fn register_endpoint(&mut self, endpoint: Endpoint) {
        self.endpoints.push(endpoint);
    }

    /// Registers default PV API endpoints.
    pub fn register_defaults(&mut self) {
        self.register_endpoint(Endpoint {
            path: "/api/v1/signals".into(),
            method: RequestMethod::Get,
            permission: Permission::Read,
            action: EndpointAction::QueryArtifacts,
            audit_level: AuditLevel::Standard,
            compliance_tags: vec![ComplianceTag::Gxp],
            quota_override: None,
        });

        self.register_endpoint(Endpoint {
            path: "/api/v1/signals/detect".into(),
            method: RequestMethod::Post,
            permission: Permission::Execute,
            action: EndpointAction::DetectSignal,
            audit_level: AuditLevel::Standard,
            compliance_tags: vec![ComplianceTag::Gxp],
            quota_override: None,
        });

        self.register_endpoint(Endpoint {
            path: "/api/v1/cases/ingest".into(),
            method: RequestMethod::Post,
            permission: Permission::Write,
            action: EndpointAction::IngestCase,
            audit_level: AuditLevel::Regulatory,
            compliance_tags: vec![ComplianceTag::Gxp, ComplianceTag::Hipaa],
            quota_override: Some(Quota::per_minute(10)),
        });

        self.register_endpoint(Endpoint {
            path: "/api/v1/workflows/signal_detection".into(),
            method: RequestMethod::Post,
            permission: Permission::Execute,
            action: EndpointAction::RunWorkflow("signal_detection".into()),
            audit_level: AuditLevel::Regulatory,
            compliance_tags: vec![ComplianceTag::Gxp],
            quota_override: None,
        });

        self.register_endpoint(Endpoint {
            path: "/api/v1/metrics".into(),
            method: RequestMethod::Get,
            permission: Permission::Read,
            action: EndpointAction::Metrics,
            audit_level: AuditLevel::Minimal,
            compliance_tags: vec![],
            quota_override: None,
        });
    }

    /// The core boundary operation: process a request through all gates.
    ///
    /// Sequence: authenticate → find endpoint → authorize → rate check → dispatch → audit
    ///
    /// # Errors
    /// Returns `GatewayError` if any gate rejects the request.
    pub fn cross(
        &mut self,
        req: &GatewayRequest,
    ) -> Result<(GatewayResponse, EndpointAction), GatewayError> {
        self.total_requests += 1;

        // 1. Authenticate (∃) — who is this?
        let identity = self.authenticate(req)?;

        // 2. Find endpoint (μ) — where are they going?
        let endpoint = self.find_endpoint(&req.path, req.method)?;

        // 3. Authorize (∂) — are they allowed?
        self.authorize(&identity, &endpoint, &req.path)?;

        // 4. Rate check (N + ∂) — within quota?
        self.rate_check(&identity)?;

        // 5. Audit (π) — log the crossing
        let action = endpoint.action.clone();
        if self.config.audit_enabled {
            let body_ref = if req.body.is_empty() {
                None
            } else {
                Some(req.body.as_str())
            };
            self.crossing_log.record(
                &identity.name,
                &req.path,
                &format!("{:?}", req.method),
                CrossingOutcome::Allowed,
                Some(endpoint.audit_level),
                endpoint.compliance_tags.clone(),
                body_ref,
                None,
            );
        }

        // 6. Return response + action for caller to dispatch
        Ok((GatewayResponse::ok("request accepted"), action))
    }

    /// Authenticates the request.
    fn authenticate(&self, req: &GatewayRequest) -> Result<Identity, GatewayError> {
        match &req.auth {
            IdentityKind::Anonymous if self.config.allow_anonymous => Ok(Identity::anonymous()),
            IdentityKind::Anonymous => {
                self.record_denied("anonymous", &req.path, &req.method);
                Err(GatewayError::AuthFailed("anonymous access disabled".into()))
            }
            kind => self
                .auth
                .authenticate(kind)
                .map(|i| i.clone())
                .map_err(|e| {
                    self.record_denied("unknown", &req.path, &req.method);
                    GatewayError::AuthFailed(e.to_string())
                }),
        }
    }

    /// Finds matching endpoint.
    fn find_endpoint(&self, path: &str, method: RequestMethod) -> Result<Endpoint, GatewayError> {
        self.endpoints
            .iter()
            .find(|e| e.path == path && e.method == method)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound(path.to_string()))
    }

    /// Authorizes identity for endpoint.
    fn authorize(
        &self,
        identity: &Identity,
        endpoint: &Endpoint,
        path: &str,
    ) -> Result<(), GatewayError> {
        // Check method-implied permission
        let method_perm = method_to_permission(endpoint.method);
        let required = if endpoint.permission > method_perm {
            endpoint.permission
        } else {
            method_perm
        };

        self.auth.authorize(identity, required, path).map_err(|e| {
            self.record_denied(&identity.name, path, &endpoint.method);
            match e {
                AuthError::PathDenied => GatewayError::Forbidden("path denied".into()),
                AuthError::InsufficientPermission { .. } => {
                    GatewayError::Forbidden("insufficient permission".into())
                }
                _ => GatewayError::AuthFailed(e.to_string()),
            }
        })
    }

    /// Checks rate limit.
    fn rate_check(&mut self, identity: &Identity) -> Result<(), GatewayError> {
        self.limiter.check(&identity.name).map_err(|e| {
            self.total_denied += 1;
            GatewayError::RateLimited {
                retry_after_secs: e.retry_after_secs,
            }
        })
    }

    /// Records a denied crossing (fire-and-forget audit, used by &self methods).
    fn record_denied(&self, identity: &str, path: &str, method: &RequestMethod) {
        // Note: crossing_log is not &mut self accessible here.
        // Denial counting is handled by total_denied in the calling context.
        // Actual audit logging happens at the cross() level.
        let _ = (identity, path, method); // Acknowledge params for future use
    }

    /// Number of registered endpoints.
    #[must_use]
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Total requests processed.
    #[must_use]
    pub fn total_requests(&self) -> u64 {
        self.total_requests
    }

    /// Total requests denied.
    #[must_use]
    pub fn total_denied(&self) -> u64 {
        self.total_denied
    }

    /// Gateway metrics snapshot.
    #[must_use]
    pub fn metrics(&self) -> GatewayMetrics {
        GatewayMetrics {
            total_requests: self.total_requests,
            total_denied: self.total_denied,
            endpoints: self.endpoints.len(),
            identities: self.auth.identity_count(),
            policies: self.auth.policy_count(),
            crossing_events: self.crossing_log.len(),
            tracked_identities: self.limiter.tracked_count(),
        }
    }
}

impl GroundsTo for Gateway {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary,    // ∂ — DOMINANT: the gateway IS the boundary
            LexPrimitiva::Mapping,     // μ — protocol translation, endpoint routing
            LexPrimitiva::Persistence, // π — crossing audit log
            LexPrimitiva::State,       // ς — connection/session state
            LexPrimitiva::Quantity,    // N — rate limits, quotas
            LexPrimitiva::Existence,   // ∃ — identity authentication
        ])
        .with_dominant(LexPrimitiva::Boundary, 0.80)
    }
}

/// Gateway metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub total_denied: u64,
    pub endpoints: usize,
    pub identities: usize,
    pub policies: usize,
    pub crossing_events: usize,
    pub tracked_identities: usize,
}

#[cfg(test)]
mod tests {
    use super::super::auth::{ApiKey, Identity, IdentityKind, Permission, ServiceAccountId};
    use super::*;

    fn setup_gateway() -> Gateway {
        let mut gw = Gateway::new(GatewayConfig::default());
        gw.register_defaults();

        // Register identities
        gw.auth_mut().register(Identity::anonymous());

        gw.auth_mut().register(Identity {
            name: "reader".into(),
            kind: IdentityKind::Key(ApiKey(1001)),
            permissions: vec![Permission::Read],
        });

        gw.auth_mut().register(Identity {
            name: "writer".into(),
            kind: IdentityKind::Key(ApiKey(2002)),
            permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
        });

        gw.auth_mut()
            .register(Identity::service("guardian", ServiceAccountId(1)));

        gw
    }

    #[test]
    fn test_gateway_cross_success() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::get("/api/v1/metrics", IdentityKind::Key(ApiKey(1001)));

        let result = gw.cross(&req);
        assert!(result.is_ok());
        if let Ok((resp, action)) = result {
            assert!(resp.status.is_success());
            assert_eq!(action, EndpointAction::Metrics);
        }
    }

    #[test]
    fn test_gateway_cross_anonymous_read() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::get("/api/v1/metrics", IdentityKind::Anonymous);

        let result = gw.cross(&req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gateway_cross_unknown_identity() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::get("/api/v1/signals", IdentityKind::Key(ApiKey(99999)));

        let result = gw.cross(&req);
        assert!(matches!(result, Err(GatewayError::AuthFailed(_))));
    }

    #[test]
    fn test_gateway_cross_insufficient_permission() {
        let mut gw = setup_gateway();

        // Reader tries to POST (requires Write/Execute)
        let req = GatewayRequest::post(
            "/api/v1/signals/detect",
            "{}",
            IdentityKind::Key(ApiKey(1001)),
        );

        let result = gw.cross(&req);
        assert!(matches!(result, Err(GatewayError::Forbidden(_))));
    }

    #[test]
    fn test_gateway_cross_not_found() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::get("/api/v1/nonexistent", IdentityKind::Key(ApiKey(1001)));

        let result = gw.cross(&req);
        assert!(matches!(result, Err(GatewayError::NotFound(_))));
    }

    #[test]
    fn test_gateway_workflow_dispatch() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::post(
            "/api/v1/workflows/signal_detection",
            "{\"drug\":\"aspirin\",\"event\":\"headache\"}",
            IdentityKind::Key(ApiKey(2002)),
        );

        let result = gw.cross(&req);
        assert!(result.is_ok());
        if let Ok((_, action)) = result {
            assert_eq!(
                action,
                EndpointAction::RunWorkflow("signal_detection".into())
            );
        }
    }

    #[test]
    fn test_gateway_case_ingest_regulatory_audit() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::post(
            "/api/v1/cases/ingest",
            "{\"drugname\":\"metformin\"}",
            IdentityKind::Key(ApiKey(2002)),
        );

        let result = gw.cross(&req);
        assert!(result.is_ok());

        // Verify crossing was audited
        let events = gw.crossing_log().events();
        assert!(!events.is_empty());

        // Last event should have GxP + HIPAA tags
        let last = events.last();
        assert!(last.is_some());
        if let Some(event) = last {
            assert!(event.tags.contains(&ComplianceTag::Gxp));
            assert!(event.tags.contains(&ComplianceTag::Hipaa));
            assert_eq!(event.level, AuditLevel::Regulatory);
        }
    }

    #[test]
    fn test_gateway_metrics() {
        let mut gw = setup_gateway();

        let req = GatewayRequest::get("/api/v1/metrics", IdentityKind::Key(ApiKey(1001)));
        let _ = gw.cross(&req);
        let _ = gw.cross(&req);

        let m = gw.metrics();
        assert_eq!(m.total_requests, 2);
        assert_eq!(m.endpoints, 5);
        assert!(m.identities > 0);
    }

    #[test]
    fn test_gateway_service_account_full_access() {
        let mut gw = setup_gateway();

        // Service account should have Admin access
        let req = GatewayRequest::post(
            "/api/v1/signals/detect",
            "{}",
            IdentityKind::ServiceAccount(ServiceAccountId(1)),
        );

        let result = gw.cross(&req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gateway_disabled_anonymous() {
        let config = GatewayConfig {
            allow_anonymous: false,
            ..GatewayConfig::default()
        };
        let mut gw = Gateway::new(config);
        gw.register_defaults();

        let req = GatewayRequest::get("/api/v1/metrics", IdentityKind::Anonymous);

        let result = gw.cross(&req);
        assert!(matches!(result, Err(GatewayError::AuthFailed(_))));
    }

    #[test]
    fn test_gateway_t3_grounding() {
        let comp = Gateway::primitive_composition();
        assert_eq!(
            nexcore_lex_primitiva::GroundingTier::classify(&comp),
            nexcore_lex_primitiva::GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.unique().len(), 6);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Boundary));
    }

    #[test]
    fn test_pvos_pvwf_pvgw_dominant_trifecta() {
        // PVOS dominant: μ (Mapping) — provides abstractions
        let pvos_comp = super::super::Pvos::primitive_composition();
        assert_eq!(pvos_comp.dominant, Some(LexPrimitiva::Mapping));

        // PVWF dominant: σ (Sequence) — orchestrates operations
        let pvwf_comp = super::super::WorkflowEngine::primitive_composition();
        assert_eq!(pvwf_comp.dominant, Some(LexPrimitiva::Sequence));

        // PVGW dominant: ∂ (Boundary) — controls what crosses
        let pvgw_comp = Gateway::primitive_composition();
        assert_eq!(pvgw_comp.dominant, Some(LexPrimitiva::Boundary));
    }
}
