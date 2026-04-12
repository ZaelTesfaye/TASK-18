use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};
use crate::config::API_BASE_URL;
use crate::store;
use crate::types::ApiError;

fn full_url(path: &str) -> String {
    format!("{}{}", API_BASE_URL, path)
}

fn apply_auth(req: Request) -> Request {
    if let Some(token) = store::get_access_token() {
        req.header("Authorization", &format!("Bearer {}", token))
    } else {
        req
    }
}

async fn handle_response<T: DeserializeOwned>(resp: gloo_net::http::Response) -> Result<T, ApiError> {
    let status = resp.status();
    if status == 401 {
        store::clear_tokens();
        return Err(ApiError {
            error: "Unauthorized".into(),
            message: "Session expired. Please log in again.".into(),
            status,
        });
    }
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(api_err) = serde_json::from_str::<ApiError>(&body) {
            return Err(ApiError {
                status,
                ..api_err
            });
        }
        return Err(ApiError {
            error: format!("HTTP {}", status),
            message: if body.is_empty() {
                format!("Request failed with status {}", status)
            } else {
                body
            },
            status,
        });
    }
    let body = resp.text().await.map_err(|e| ApiError {
        error: "Parse error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    serde_json::from_str::<T>(&body).map_err(|e| ApiError {
        error: "Deserialization error".into(),
        message: format!("{} — body: {}", e, &body[..body.len().min(200)]),
        status: 0,
    })
}

async fn handle_empty_response(resp: gloo_net::http::Response) -> Result<(), ApiError> {
    let status = resp.status();
    if status == 401 {
        store::clear_tokens();
        return Err(ApiError {
            error: "Unauthorized".into(),
            message: "Session expired. Please log in again.".into(),
            status,
        });
    }
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(api_err) = serde_json::from_str::<ApiError>(&body) {
            return Err(ApiError { status, ..api_err });
        }
        return Err(ApiError {
            error: format!("HTTP {}", status),
            message: if body.is_empty() {
                format!("Request failed with status {}", status)
            } else {
                body
            },
            status,
        });
    }
    Ok(())
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let url = full_url(path);
    let req = apply_auth(Request::get(&url));
    let resp = req.send().await.map_err(|e| ApiError {
        error: "Network error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    handle_response(resp).await
}

pub async fn post<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, ApiError> {
    let url = full_url(path);
    let req = apply_auth(Request::post(&url))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap_or_default())
        .map_err(|e| ApiError {
            error: "Request build error".into(),
            message: e.to_string(),
            status: 0,
        })?;
    let resp = req.send().await.map_err(|e| ApiError {
        error: "Network error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    handle_response(resp).await
}

pub async fn put<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, ApiError> {
    let url = full_url(path);
    let req = apply_auth(Request::put(&url))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap_or_default())
        .map_err(|e| ApiError {
            error: "Request build error".into(),
            message: e.to_string(),
            status: 0,
        })?;
    let resp = req.send().await.map_err(|e| ApiError {
        error: "Network error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    handle_response(resp).await
}

pub async fn delete(path: &str) -> Result<(), ApiError> {
    let url = full_url(path);
    let req = apply_auth(Request::delete(&url));
    let resp = req.send().await.map_err(|e| ApiError {
        error: "Network error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    handle_empty_response(resp).await
}

pub async fn post_empty<B: Serialize>(path: &str, body: &B) -> Result<(), ApiError> {
    let url = full_url(path);
    let req = apply_auth(Request::post(&url))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap_or_default())
        .map_err(|e| ApiError {
            error: "Request build error".into(),
            message: e.to_string(),
            status: 0,
        })?;
    let resp = req.send().await.map_err(|e| ApiError {
        error: "Network error".into(),
        message: e.to_string(),
        status: 0,
    })?;
    handle_empty_response(resp).await
}
