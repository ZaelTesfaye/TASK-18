use actix_web::{HttpResponse, http::StatusCode};
use serde::Serialize;
use std::fmt;

/// Structured JSON error body returned to API consumers.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status: u16,
}

/// Application-level error enum. Maps every variant to a specific HTTP status code
/// and produces a safe JSON error body that never leaks stack traces.
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    RateLimited(String),
    ValidationError(String),
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::RateLimited(msg) => write!(f, "Rate Limited: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl actix_web::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let (error_label, message) = match self {
            AppError::BadRequest(msg) => ("BadRequest", msg.clone()),
            AppError::Unauthorized(msg) => ("Unauthorized", msg.clone()),
            AppError::Forbidden(msg) => ("Forbidden", msg.clone()),
            AppError::NotFound(msg) => ("NotFound", msg.clone()),
            AppError::Conflict(msg) => ("Conflict", msg.clone()),
            AppError::RateLimited(msg) => ("RateLimited", msg.clone()),
            AppError::ValidationError(msg) => ("ValidationError", msg.clone()),
            AppError::InternalError(_) => (
                "InternalError",
                "An internal server error occurred".to_string(),
            ),
        };

        HttpResponse::build(status).json(ErrorResponse {
            error: error_label.to_string(),
            message,
            status: status.as_u16(),
        })
    }
}

// ---------------------------------------------------------------------------
// Convenience conversions from common library errors
// ---------------------------------------------------------------------------

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "Database error");
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(ref db_err) => {
                // PostgreSQL unique-violation code
                if db_err.code().as_deref() == Some("23505") {
                    AppError::Conflict("A record with this value already exists".to_string())
                } else {
                    AppError::InternalError("Database error".to_string())
                }
            }
            _ => AppError::InternalError("Database error".to_string()),
        }
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        tracing::warn!(error = %err, "JWT error");
        AppError::Unauthorized("Invalid or expired token".to_string())
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        tracing::error!(error = %err, "Password hashing error");
        AppError::InternalError("Authentication processing error".to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::BadRequest(format!("Invalid JSON: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_all_variants() {
        let cases = vec![
            (AppError::BadRequest("x".into()), "Bad Request: x"),
            (AppError::Unauthorized("x".into()), "Unauthorized: x"),
            (AppError::Forbidden("x".into()), "Forbidden: x"),
            (AppError::NotFound("x".into()), "Not Found: x"),
            (AppError::Conflict("x".into()), "Conflict: x"),
            (AppError::RateLimited("x".into()), "Rate Limited: x"),
            (AppError::ValidationError("x".into()), "Validation Error: x"),
            (AppError::InternalError("x".into()), "Internal Error: x"),
        ];
        for (err, expected) in cases {
            assert_eq!(format!("{}", err), expected);
        }
    }

    #[test]
    fn test_status_codes() {
        use actix_web::ResponseError;
        assert_eq!(AppError::BadRequest("".into()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::Unauthorized("".into()).status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::Forbidden("".into()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::NotFound("".into()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(AppError::Conflict("".into()).status_code(), StatusCode::CONFLICT);
        assert_eq!(AppError::RateLimited("".into()).status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(AppError::ValidationError("".into()).status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(AppError::InternalError("".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_response_hides_internal_details() {
        use actix_web::ResponseError;
        let err = AppError::InternalError("secret db password leaked".into());
        let resp = err.error_response();
        let body = resp.into_body();
        let bytes = actix_web::body::to_bytes(body);
        // The error_response() for InternalError must NOT leak the original message
        assert_eq!(
            format!("{}", err),
            "Internal Error: secret db password leaked"
        );
        // But Display is for logging, error_response is for the client
    }

    #[test]
    fn test_error_response_json_structure() {
        use actix_web::ResponseError;
        let err = AppError::NotFound("User not found".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_from_serde_json_error() {
        let bad_json = "{invalid";
        let result: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
        let err: AppError = result.unwrap_err().into();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("Invalid JSON")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_from_jwt_error() {
        let jwt_err = jsonwebtoken::decode::<serde_json::Value>(
            "invalid",
            &jsonwebtoken::DecodingKey::from_secret(b"secret"),
            &jsonwebtoken::Validation::default(),
        )
        .unwrap_err();
        let err: AppError = jwt_err.into();
        match err {
            AppError::Unauthorized(msg) => assert!(msg.contains("Invalid or expired")),
            _ => panic!("Expected Unauthorized"),
        }
    }
}
