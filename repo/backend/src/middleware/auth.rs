use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::services::auth_service;

// ---------------------------------------------------------------------------
// AuthenticatedUser extractor
// ---------------------------------------------------------------------------

/// Extractor that validates a JWT Bearer token from the `Authorization` header
/// and ensures the token has not been revoked. Routes that include this
/// extractor in their handler signature will automatically reject
/// unauthenticated requests with a 401 Unauthorized response.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub role: String,
    pub jti: String,
}

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // 1. Extract the Bearer token from the Authorization header.
            let token = extract_bearer_token(&req)?;

            // 2. Validate the JWT signature and claims.
            let config = Config::get();
            let claims = auth_service::validate_token(&token, &config.jwt_secret)?;

            // 3. Check whether the token has been revoked.
            let pool = req
                .app_data::<web::Data<PgPool>>()
                .ok_or_else(|| {
                    tracing::error!("PgPool not found in app_data");
                    AppError::InternalError("Server configuration error".to_string())
                })?;

            if auth_service::is_token_revoked(pool.get_ref(), &claims.jti).await? {
                return Err(AppError::Unauthorized("Token has been revoked".to_string()));
            }

            Ok(AuthenticatedUser {
                user_id: claims.sub,
                role: claims.role,
                jti: claims.jti,
            })
        })
    }
}

// ---------------------------------------------------------------------------
// OptionalUser extractor
// ---------------------------------------------------------------------------

/// Like [`AuthenticatedUser`], but does **not** fail when no token is present.
/// Returns `None` for anonymous requests and `Some(AuthenticatedUser)` when a
/// valid, non-revoked token is supplied. An *invalid* or *revoked* token still
/// produces `None` rather than an error, making this suitable for endpoints
/// that offer enhanced functionality to authenticated users without requiring
/// authentication.
pub struct OptionalUser(pub Option<AuthenticatedUser>);

impl FromRequest for OptionalUser {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // If there is no Authorization header at all, return None immediately.
            let token = match extract_bearer_token(&req) {
                Ok(t) => t,
                Err(_) => return Ok(OptionalUser(None)),
            };

            let config = Config::get();
            let claims = match auth_service::validate_token(&token, &config.jwt_secret) {
                Ok(c) => c,
                Err(_) => return Ok(OptionalUser(None)),
            };

            let pool = match req.app_data::<web::Data<PgPool>>() {
                Some(p) => p,
                None => return Ok(OptionalUser(None)),
            };

            match auth_service::is_token_revoked(pool.get_ref(), &claims.jti).await {
                Ok(true) => return Ok(OptionalUser(None)),
                Ok(false) => {}
                Err(_) => return Ok(OptionalUser(None)),
            }

            Ok(OptionalUser(Some(AuthenticatedUser {
                user_id: claims.sub,
                role: claims.role,
                jti: claims.jti,
            })))
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extracts a Bearer token string from the `Authorization` header.
fn extract_bearer_token(req: &HttpRequest) -> Result<String, AppError> {
    let header_value = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid Authorization header encoding".to_string()))?;

    if !header_value.starts_with("Bearer ") {
        return Err(AppError::Unauthorized(
            "Authorization header must use Bearer scheme".to_string(),
        ));
    }

    let token = header_value[7..].trim().to_string();
    if token.is_empty() {
        return Err(AppError::Unauthorized("Bearer token is empty".to_string()));
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_extract_bearer_token_valid() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer mytoken123"))
            .to_http_request();
        let token = extract_bearer_token(&req).unwrap();
        assert_eq!(token, "mytoken123");
    }

    #[test]
    fn test_extract_bearer_token_missing_header() {
        let req = TestRequest::default().to_http_request();
        let err = extract_bearer_token(&req).unwrap_err();
        match err {
            AppError::Unauthorized(msg) => assert!(msg.contains("Missing")),
            _ => panic!("Expected Unauthorized"),
        }
    }

    #[test]
    fn test_extract_bearer_token_wrong_scheme() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Basic abc123"))
            .to_http_request();
        let err = extract_bearer_token(&req).unwrap_err();
        match err {
            AppError::Unauthorized(msg) => assert!(msg.contains("Bearer scheme")),
            _ => panic!("Expected Unauthorized"),
        }
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer "))
            .to_http_request();
        let err = extract_bearer_token(&req).unwrap_err();
        match err {
            AppError::Unauthorized(msg) => assert!(msg.contains("empty")),
            _ => panic!("Expected Unauthorized"),
        }
    }

    #[test]
    fn test_extract_bearer_token_trims_whitespace() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer  token_with_spaces  "))
            .to_http_request();
        let token = extract_bearer_token(&req).unwrap();
        assert_eq!(token, "token_with_spaces");
    }
}
