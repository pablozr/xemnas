//! HTTP error mapping: product-language body, technical detail only in `tracing`.
//!
//! Every response body is `{"code": "...", "message": "<PT-BR>"}`. The store and
//! validation details never reach the client; they are logged instead.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// The error surface of the local API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApiError {
    /// Malformed JSON, missing/divergent `Idempotency-Key`, wrong content type.
    BadRequest,
    /// Missing or invalid bearer token.
    Unauthorized,
    /// Browser `Origin`, unregistered project or non-canonicalizable path.
    Forbidden,
    /// Unknown capture receipt.
    NotFound,
    /// Duplicate artifact identifier in the payload (transaction rolled back).
    Conflict,
    /// Body exceeded the configured limit.
    PayloadTooLarge,
    /// Envelope failed JSON Schema validation or deserialization.
    Unprocessable,
    /// The request exceeded the configured timeout.
    GatewayTimeout,
    /// Storage or internal failure; the detail is logged, not returned.
    Internal,
}

impl ApiError {
    /// HTTP status for this error.
    pub(crate) fn status(self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Unprocessable => StatusCode::UNPROCESSABLE_ENTITY,
            Self::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine code for the error body.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::PayloadTooLarge => "payload_too_large",
            Self::Unprocessable => "unprocessable_entity",
            Self::GatewayTimeout => "gateway_timeout",
            Self::Internal => "internal_error",
        }
    }

    /// Product-language message shown to the caller.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::BadRequest => "requisição inválida",
            Self::Unauthorized => "token ausente ou inválido",
            Self::Forbidden => "acesso negado",
            Self::NotFound => "captura não encontrada",
            Self::Conflict => "artefato repetido no payload",
            Self::PayloadTooLarge => "corpo da requisição excede o limite",
            Self::Unprocessable => "envelope de captura inválido",
            Self::GatewayTimeout => "tempo de processamento excedido",
            Self::Internal => "falha interna",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "code": self.code(),
            "message": self.message(),
        }));
        let mut response = (self.status(), body).into_response();
        if matches!(self, Self::Unauthorized) {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;
    use axum::http::StatusCode;

    #[test]
    fn statuses_and_codes_cover_the_ticket_matrix() {
        assert_eq!(ApiError::BadRequest.status(), StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::Unauthorized.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::Forbidden.status(), StatusCode::FORBIDDEN);
        assert_eq!(ApiError::NotFound.status(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::Conflict.status(), StatusCode::CONFLICT);
        assert_eq!(
            ApiError::PayloadTooLarge.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            ApiError::Unprocessable.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::GatewayTimeout.status(),
            StatusCode::GATEWAY_TIMEOUT
        );
        assert_eq!(
            ApiError::Internal.status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(ApiError::Forbidden.code(), "forbidden");
        assert_eq!(ApiError::Unauthorized.code(), "unauthorized");
    }
}
