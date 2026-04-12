use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;

// ---------------------------------------------------------------------------
// Role guards
// ---------------------------------------------------------------------------

/// Asserts that the authenticated user holds exactly the given role.
///
/// Returns `Err(AppError::Forbidden)` when the user's role does not match.
///
/// # Example
/// ```ignore
/// require_role(&user, "Admin")?;
/// ```
pub fn require_role(user: &AuthenticatedUser, required: &str) -> Result<(), AppError> {
    if user.role != required {
        tracing::warn!(
            user_id = %user.user_id,
            user_role = %user.role,
            required_role = %required,
            "Access denied: role mismatch"
        );
        return Err(AppError::Forbidden(format!(
            "This action requires the '{}' role",
            required
        )));
    }
    Ok(())
}

/// Asserts that the authenticated user holds **any one** of the given roles.
///
/// Returns `Err(AppError::Forbidden)` when the user's role is not in the list.
///
/// # Example
/// ```ignore
/// require_any_role(&user, &["Admin", "Reviewer"])?;
/// ```
pub fn require_any_role(user: &AuthenticatedUser, roles: &[&str]) -> Result<(), AppError> {
    if roles.iter().any(|&r| user.role == r) {
        return Ok(());
    }

    tracing::warn!(
        user_id = %user.user_id,
        user_role = %user.role,
        allowed_roles = ?roles,
        "Access denied: none of the required roles matched"
    );
    Err(AppError::Forbidden(format!(
        "This action requires one of the following roles: {}",
        roles.join(", ")
    )))
}

/// Asserts that the authenticated user either owns the resource **or** is an Admin.
///
/// Useful for endpoints where users may view/edit their own resources but
/// administrators can access anyone's.
///
/// # Example
/// ```ignore
/// require_owner_or_admin(&user, order.user_id)?;
/// ```
pub fn require_owner_or_admin(
    user: &AuthenticatedUser,
    resource_owner_id: Uuid,
) -> Result<(), AppError> {
    if user.role == "Admin" || user.user_id == resource_owner_id {
        return Ok(());
    }

    tracing::warn!(
        user_id = %user.user_id,
        resource_owner_id = %resource_owner_id,
        "Access denied: user is neither the owner nor an Admin"
    );
    Err(AppError::Forbidden(
        "You do not have permission to access this resource".to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(role: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: Uuid::new_v4(),
            role: role.to_string(),
            jti: "test-jti".to_string(),
        }
    }

    #[test]
    fn require_role_accepts_matching_role() {
        let user = make_user("Admin");
        assert!(require_role(&user, "Admin").is_ok());
    }

    #[test]
    fn require_role_rejects_non_matching_role() {
        let user = make_user("Shopper");
        assert!(require_role(&user, "Admin").is_err());
    }

    #[test]
    fn require_any_role_accepts_when_role_in_list() {
        let user = make_user("Reviewer");
        assert!(require_any_role(&user, &["Admin", "Reviewer"]).is_ok());
    }

    #[test]
    fn require_any_role_rejects_when_role_not_in_list() {
        let user = make_user("Shopper");
        assert!(require_any_role(&user, &["Admin", "Reviewer"]).is_err());
    }

    #[test]
    fn require_owner_or_admin_accepts_owner() {
        let user = make_user("Shopper");
        assert!(require_owner_or_admin(&user, user.user_id).is_ok());
    }

    #[test]
    fn require_owner_or_admin_accepts_admin() {
        let user = make_user("Admin");
        let other_id = Uuid::new_v4();
        assert!(require_owner_or_admin(&user, other_id).is_ok());
    }

    #[test]
    fn require_owner_or_admin_rejects_non_owner_non_admin() {
        let user = make_user("Shopper");
        let other_id = Uuid::new_v4();
        assert!(require_owner_or_admin(&user, other_id).is_err());
    }
}
