use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use futures::future::{ok, Ready};
use std::future::Future;
use std::pin::Pin;

/// Middleware that logs incoming requests with redacted sensitive fields.
pub struct RequestLogger;

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = RequestLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestLoggerMiddleware { service })
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();

        tracing::info!(method = %method, path = %path, "Incoming request");

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            tracing::info!(
                method = %method,
                path = %path,
                status = %res.status().as_u16(),
                "Request completed"
            );
            Ok(res)
        })
    }
}

/// Redacts sensitive fields from log output.
pub fn redact_sensitive(value: &str) -> String {
    if value.len() <= 4 {
        return "****".to_string();
    }
    format!("{}****", &value[..2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_short_value() {
        assert_eq!(redact_sensitive("abc"), "****");
        assert_eq!(redact_sensitive(""), "****");
        assert_eq!(redact_sensitive("1234"), "****");
    }

    #[test]
    fn test_redact_long_value() {
        assert_eq!(redact_sensitive("mysecret"), "my****");
        assert_eq!(redact_sensitive("password123"), "pa****");
    }

    #[test]
    fn test_redact_preserves_first_two_chars() {
        let result = redact_sensitive("Hello World");
        assert!(result.starts_with("He"));
        assert!(result.ends_with("****"));
    }
}
