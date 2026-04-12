use gloo::storage::{LocalStorage, Storage};
use crate::types::UserResponse;

const ACCESS_TOKEN_KEY: &str = "silverscreen_access_token";
const REFRESH_TOKEN_KEY: &str = "silverscreen_refresh_token";
const USER_KEY: &str = "silverscreen_user";

pub fn save_tokens(access: &str, refresh: &str) {
    let _ = LocalStorage::set(ACCESS_TOKEN_KEY, access.to_string());
    let _ = LocalStorage::set(REFRESH_TOKEN_KEY, refresh.to_string());
}

pub fn get_access_token() -> Option<String> {
    LocalStorage::get::<String>(ACCESS_TOKEN_KEY).ok()
}

pub fn get_refresh_token() -> Option<String> {
    LocalStorage::get::<String>(REFRESH_TOKEN_KEY).ok()
}

pub fn clear_tokens() {
    LocalStorage::delete(ACCESS_TOKEN_KEY);
    LocalStorage::delete(REFRESH_TOKEN_KEY);
    LocalStorage::delete(USER_KEY);
}

pub fn save_user(user: &UserResponse) {
    if let Ok(json) = serde_json::to_string(user) {
        let _ = LocalStorage::set(USER_KEY, json);
    }
}

pub fn get_user() -> Option<UserResponse> {
    LocalStorage::get::<String>(USER_KEY)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub fn is_authenticated() -> bool {
    get_access_token().is_some()
}

pub fn get_role() -> Option<String> {
    get_user().map(|u| u.role)
}
