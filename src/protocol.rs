//! # PVGW Protocol Translation
//!
//! Maps between external wire protocols and internal gateway representations.
//! The gateway speaks one internal language; translators adapt external formats.
//!
//! ## Primitive: μ (Mapping)
//!
//! Protocol translation is pure mapping — converting between representations
//! without altering semantics.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::auth::Permission;

/// Wire protocol used by the caller.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// RESTful HTTP API.
    Rest,
    /// gRPC (future).
    Grpc,
    /// Event/message bus (pub/sub).
    Event,
    /// Internal (in-process) call.
    Internal,
}

impl GroundsTo for Protocol {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

/// HTTP-like request method.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RequestMethod {
    /// Read data.
    Get,
    /// Create/submit data.
    Post,
    /// Update existing data.
    Put,
    /// Remove data.
    Delete,
    /// Subscribe to events.
    Subscribe,
    /// Publish an event.
    Publish,
}

impl GroundsTo for RequestMethod {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

/// Maps request methods to minimum required permission.
/// Tier: μ (pure mapping)
///
/// GET/Subscribe → Read
/// POST/Publish → Write
/// PUT → Write
/// DELETE → Admin
#[must_use]
pub fn method_to_permission(method: RequestMethod) -> Permission {
    match method {
        RequestMethod::Get | RequestMethod::Subscribe => Permission::Read,
        RequestMethod::Post | RequestMethod::Publish | RequestMethod::Put => Permission::Write,
        RequestMethod::Delete => Permission::Admin,
    }
}

/// HTTP status code categories for response translation.
/// Tier: T2-P (μ + N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusCode {
    /// 200 OK — request succeeded.
    Ok,
    /// 201 Created — resource created.
    Created,
    /// 400 Bad Request — invalid input.
    BadRequest,
    /// 401 Unauthorized — auth failed.
    Unauthorized,
    /// 403 Forbidden — insufficient permission.
    Forbidden,
    /// 404 Not Found — endpoint not found.
    NotFound,
    /// 429 Too Many Requests — rate limited.
    TooManyRequests,
    /// 500 Internal Server Error.
    InternalError,
}

impl StatusCode {
    /// Converts to numeric HTTP status code.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::Created => 201,
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::TooManyRequests => 429,
            Self::InternalError => 500,
        }
    }

    /// Creates from numeric HTTP status code.
    #[must_use]
    pub fn from_u16(code: u16) -> Self {
        match code {
            200 => Self::Ok,
            201 => Self::Created,
            400 => Self::BadRequest,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            429 => Self::TooManyRequests,
            _ => Self::InternalError,
        }
    }

    /// Returns true if this is a success status.
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Ok | Self::Created)
    }
}

impl GroundsTo for StatusCode {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Quantity])
    }
}

/// Parses a REST method string into RequestMethod.
///
/// # Errors
/// Returns `None` for unknown methods.
#[must_use]
pub fn parse_method(s: &str) -> Option<RequestMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Some(RequestMethod::Get),
        "POST" => Some(RequestMethod::Post),
        "PUT" => Some(RequestMethod::Put),
        "DELETE" => Some(RequestMethod::Delete),
        _ => None,
    }
}

/// Content type for request/response bodies.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    /// JSON payload.
    Json,
    /// Form URL-encoded.
    FormUrlEncoded,
    /// Plain text.
    PlainText,
    /// No body.
    None,
}

impl GroundsTo for ContentType {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_method_to_permission_read() {
        assert_eq!(method_to_permission(RequestMethod::Get), Permission::Read);
        assert_eq!(
            method_to_permission(RequestMethod::Subscribe),
            Permission::Read
        );
    }

    #[test]
    fn test_method_to_permission_write() {
        assert_eq!(method_to_permission(RequestMethod::Post), Permission::Write);
        assert_eq!(method_to_permission(RequestMethod::Put), Permission::Write);
        assert_eq!(
            method_to_permission(RequestMethod::Publish),
            Permission::Write
        );
    }

    #[test]
    fn test_method_to_permission_admin() {
        assert_eq!(
            method_to_permission(RequestMethod::Delete),
            Permission::Admin
        );
    }

    #[test]
    fn test_status_code_roundtrip() {
        let codes = [200, 201, 400, 401, 403, 404, 429, 500];
        for code in codes {
            let status = StatusCode::from_u16(code);
            assert_eq!(status.as_u16(), code);
        }
    }

    #[test]
    fn test_status_code_success() {
        assert!(StatusCode::Ok.is_success());
        assert!(StatusCode::Created.is_success());
        assert!(!StatusCode::BadRequest.is_success());
        assert!(!StatusCode::Unauthorized.is_success());
        assert!(!StatusCode::InternalError.is_success());
    }

    #[test]
    fn test_parse_method() {
        assert_eq!(parse_method("GET"), Some(RequestMethod::Get));
        assert_eq!(parse_method("post"), Some(RequestMethod::Post));
        assert_eq!(parse_method("Put"), Some(RequestMethod::Put));
        assert_eq!(parse_method("DELETE"), Some(RequestMethod::Delete));
        assert_eq!(parse_method("PATCH"), None);
    }

    #[test]
    fn test_protocol_grounding() {
        let comp = Protocol::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
    }

    #[test]
    fn test_status_code_grounding() {
        let comp = StatusCode::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
