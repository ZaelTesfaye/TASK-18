use crate::api::client;
use crate::types::*;

pub async fn list_products(filter: &ProductFilter) -> Result<PaginatedResponse<Product>, ApiError> {
    let mut params = Vec::new();
    if let Some(ref s) = filter.search {
        if !s.is_empty() {
            params.push(format!("search={}", urlencoding(s)));
        }
    }
    if let Some(ref g) = filter.genre {
        if !g.is_empty() {
            params.push(format!("genre={}", urlencoding(g)));
        }
    }
    if let Some(ref t) = filter.topic_id {
        if !t.is_empty() {
            params.push(format!("topic_id={}", urlencoding(t)));
        }
    }
    if let Some(ref t) = filter.tag_id {
        if !t.is_empty() {
            params.push(format!("tag_id={}", urlencoding(t)));
        }
    }
    if let Some(v) = filter.min_price {
        params.push(format!("min_price={}", v));
    }
    if let Some(v) = filter.max_price {
        params.push(format!("max_price={}", v));
    }
    if let Some(p) = filter.page {
        params.push(format!("page={}", p));
    }
    if let Some(ref cfn) = filter.custom_field_name {
        if !cfn.is_empty() {
            params.push(format!("custom_field_name={}", urlencoding(cfn)));
        }
    }
    if let Some(ref cfv) = filter.custom_field_value {
        if !cfv.is_empty() {
            params.push(format!("custom_field_value={}", urlencoding(cfv)));
        }
    }
    if let Some(pp) = filter.per_page {
        params.push(format!("per_page={}", pp));
    }
    let qs = if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    };
    client::get(&format!("/products{}", qs)).await
}

pub async fn get_product(id: &str) -> Result<Product, ApiError> {
    client::get(&format!("/products/{}", id)).await
}

/// Simple percent-encoding for query parameters.
fn urlencoding(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_default()
}
