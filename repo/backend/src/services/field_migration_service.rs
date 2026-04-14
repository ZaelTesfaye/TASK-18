use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::custom_field::CustomFieldValue;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Supported custom field types, mirroring the Postgres `field_type` enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    Enum,
    Date,
    Number,
}

impl FieldType {
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "Text" => Ok(FieldType::Text),
            "Enum" => Ok(FieldType::Enum),
            "Date" => Ok(FieldType::Date),
            "Number" => Ok(FieldType::Number),
            _ => Err(AppError::BadRequest(format!("Unknown field type: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "Text",
            FieldType::Enum => "Enum",
            FieldType::Date => "Date",
            FieldType::Number => "Number",
        }
    }
}

/// Status of a single field value in the migration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationItemStatus {
    AutoConverted,
    Conflict { message: String },
}

/// A single item in the migration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlanItem {
    pub product_id: Uuid,
    pub current_value: serde_json::Value,
    pub converted_value: Option<serde_json::Value>,
    pub status: MigrationItemStatus,
}

/// The complete migration plan for a field type change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub field_id: Uuid,
    pub new_type: String,
    pub new_allowed: Option<Vec<String>>,
    pub total_values: usize,
    pub auto_converted: usize,
    pub conflicts: usize,
    pub items: Vec<MigrationPlanItem>,
}

// ---------------------------------------------------------------------------
// Plan migration
// ---------------------------------------------------------------------------

/// Plans a migration of a custom field from its current type to `new_type`.
///
/// For each product that has a value for this field, attempts to convert the
/// existing value to the new type. Compatible values are marked `AutoConverted`;
/// incompatible values are marked `Conflict` with an explanation.
pub async fn plan_migration(
    pool: &PgPool,
    field_id: Uuid,
    new_type: FieldType,
    new_allowed: Option<Vec<String>>,
) -> Result<MigrationPlan, AppError> {
    let values = sqlx::query_as::<_, CustomFieldValue>(
        "SELECT * FROM custom_field_values WHERE field_id = $1",
    )
    .bind(field_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch field values: {}", e))
    })?;

    let mut items = Vec::new();
    let mut auto_converted = 0usize;
    let mut conflicts = 0usize;

    for fv in &values {
        match try_convert(&fv.value, &new_type, &new_allowed) {
            Ok(converted) => {
                items.push(MigrationPlanItem {
                    product_id: fv.product_id,
                    current_value: fv.value.clone(),
                    converted_value: Some(converted),
                    status: MigrationItemStatus::AutoConverted,
                });
                auto_converted += 1;
            }
            Err(msg) => {
                items.push(MigrationPlanItem {
                    product_id: fv.product_id,
                    current_value: fv.value.clone(),
                    converted_value: None,
                    status: MigrationItemStatus::Conflict { message: msg },
                });
                conflicts += 1;
            }
        }
    }

    Ok(MigrationPlan {
        field_id,
        new_type: new_type.as_str().to_string(),
        new_allowed,
        total_values: values.len(),
        auto_converted,
        conflicts,
        items,
    })
}

/// Attempts to convert a JSON value to the target type.
fn try_convert(
    value: &serde_json::Value,
    new_type: &FieldType,
    new_allowed: &Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let raw = match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => {
            return Ok(serde_json::Value::Null);
        }
        _ => return Err("Cannot convert complex JSON value".to_string()),
    };

    match new_type {
        FieldType::Text => Ok(serde_json::Value::String(raw)),

        FieldType::Number => raw
            .parse::<f64>()
            .map(|n| {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(raw.clone()))
            })
            .map_err(|_| format!("Cannot convert '{}' to Number", raw)),

        FieldType::Date => {
            // Accept ISO 8601 date format
            if chrono::NaiveDate::parse_from_str(&raw, "%Y-%m-%d").is_ok() {
                Ok(serde_json::Value::String(raw))
            } else if chrono::DateTime::parse_from_rfc3339(&raw).is_ok() {
                Ok(serde_json::Value::String(raw))
            } else {
                Err(format!(
                    "Cannot convert '{}' to Date (expected YYYY-MM-DD or RFC3339)",
                    raw
                ))
            }
        }

        FieldType::Enum => {
            if let Some(allowed) = new_allowed {
                if allowed.contains(&raw) {
                    Ok(serde_json::Value::String(raw))
                } else {
                    Err(format!(
                        "Value '{}' is not in the allowed list: {:?}",
                        raw, allowed
                    ))
                }
            } else {
                Ok(serde_json::Value::String(raw))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Resolve conflict
// ---------------------------------------------------------------------------

/// Manually resolves a migration conflict by setting a new value for a specific
/// product field and decrementing the conflict count on the field definition.
pub async fn resolve_conflict(
    pool: &PgPool,
    field_id: Uuid,
    product_id: Uuid,
    new_value: serde_json::Value,
) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Update the field value
    let rows_affected = sqlx::query(
        "UPDATE custom_field_values SET value = $1, conflict_status = 'Resolved', \
         conflict_message = NULL, updated_at = NOW() \
         WHERE field_id = $2 AND product_id = $3",
    )
    .bind(&new_value)
    .bind(field_id)
    .bind(product_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to update field value: {}", e))
    })?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(
            "No field value found for this product/field combination".to_string(),
        ));
    }

    // Decrement conflict count (floor at 0)
    sqlx::query(
        "UPDATE custom_field_definitions SET conflict_count = GREATEST(conflict_count - 1, 0), \
         updated_at = NOW() WHERE id = $1",
    )
    .bind(field_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to update conflict count: {}", e))
    })?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    log::info!(
        "Migration conflict resolved: field_id={}, product_id={}",
        field_id,
        product_id
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Publish check
// ---------------------------------------------------------------------------

/// Returns `true` if the field has zero remaining conflicts and is safe to
/// publish.
pub async fn can_publish(pool: &PgPool, field_id: Uuid) -> Result<bool, AppError> {
    let count = sqlx::query_scalar::<_, i32>(
        "SELECT conflict_count FROM custom_field_definitions WHERE id = $1",
    )
    .bind(field_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch field definition: {}", e))
    })?
    .ok_or_else(|| AppError::NotFound("Field definition not found".to_string()))?;

    Ok(count == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- FieldType parsing --

    #[test]
    fn test_field_type_from_str_all_valid() {
        assert_eq!(FieldType::from_str("Text").unwrap(), FieldType::Text);
        assert_eq!(FieldType::from_str("Enum").unwrap(), FieldType::Enum);
        assert_eq!(FieldType::from_str("Date").unwrap(), FieldType::Date);
        assert_eq!(FieldType::from_str("Number").unwrap(), FieldType::Number);
    }

    #[test]
    fn test_field_type_from_str_invalid() {
        assert!(FieldType::from_str("Invalid").is_err());
        assert!(FieldType::from_str("text").is_err()); // case sensitive
    }

    #[test]
    fn test_field_type_roundtrip() {
        for ft in &[FieldType::Text, FieldType::Enum, FieldType::Date, FieldType::Number] {
            assert_eq!(FieldType::from_str(ft.as_str()).unwrap(), *ft);
        }
    }

    // -- try_convert to Text --

    #[test]
    fn test_convert_number_to_text() {
        let val = serde_json::json!(42);
        let result = try_convert(&val, &FieldType::Text, &None).unwrap();
        assert_eq!(result, serde_json::json!("42"));
    }

    #[test]
    fn test_convert_string_to_text() {
        let val = serde_json::json!("hello");
        let result = try_convert(&val, &FieldType::Text, &None).unwrap();
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[test]
    fn test_convert_bool_to_text() {
        let val = serde_json::json!(true);
        let result = try_convert(&val, &FieldType::Text, &None).unwrap();
        assert_eq!(result, serde_json::json!("true"));
    }

    #[test]
    fn test_convert_null_passes_through() {
        let val = serde_json::Value::Null;
        let result = try_convert(&val, &FieldType::Number, &None).unwrap();
        assert_eq!(result, serde_json::Value::Null);
    }

    // -- try_convert to Number --

    #[test]
    fn test_convert_string_number_to_number() {
        let val = serde_json::json!("42.5");
        let result = try_convert(&val, &FieldType::Number, &None).unwrap();
        assert_eq!(result, serde_json::json!(42.5));
    }

    #[test]
    fn test_convert_non_numeric_string_to_number_fails() {
        let val = serde_json::json!("not_a_number");
        let result = try_convert(&val, &FieldType::Number, &None);
        assert!(result.is_err());
    }

    // -- try_convert to Date --

    #[test]
    fn test_convert_valid_date_string() {
        let val = serde_json::json!("2024-06-15");
        let result = try_convert(&val, &FieldType::Date, &None).unwrap();
        assert_eq!(result, serde_json::json!("2024-06-15"));
    }

    #[test]
    fn test_convert_rfc3339_date_string() {
        let val = serde_json::json!("2024-06-15T10:30:00+00:00");
        let result = try_convert(&val, &FieldType::Date, &None).unwrap();
        assert!(result.is_string());
    }

    #[test]
    fn test_convert_invalid_date_fails() {
        let val = serde_json::json!("not-a-date");
        let result = try_convert(&val, &FieldType::Date, &None);
        assert!(result.is_err());
    }

    // -- try_convert to Enum --

    #[test]
    fn test_convert_to_enum_in_allowed_list() {
        let allowed = Some(vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()]);
        let val = serde_json::json!("Green");
        let result = try_convert(&val, &FieldType::Enum, &allowed).unwrap();
        assert_eq!(result, serde_json::json!("Green"));
    }

    #[test]
    fn test_convert_to_enum_not_in_allowed_list() {
        let allowed = Some(vec!["Red".to_string(), "Green".to_string()]);
        let val = serde_json::json!("Purple");
        let result = try_convert(&val, &FieldType::Enum, &allowed);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_enum_no_allowed_list() {
        let val = serde_json::json!("anything");
        let result = try_convert(&val, &FieldType::Enum, &None).unwrap();
        assert_eq!(result, serde_json::json!("anything"));
    }

    // -- Complex values --

    #[test]
    fn test_convert_object_fails() {
        let val = serde_json::json!({"nested": "object"});
        let result = try_convert(&val, &FieldType::Text, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_array_fails() {
        let val = serde_json::json!([1, 2, 3]);
        let result = try_convert(&val, &FieldType::Text, &None);
        assert!(result.is_err());
    }
}
