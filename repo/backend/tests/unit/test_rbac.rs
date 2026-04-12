use silverscreen_backend::middleware::auth::AuthenticatedUser;
use silverscreen_backend::middleware::rbac::{require_any_role, require_owner_or_admin, require_role};
use uuid::Uuid;

fn make_user(role: &str) -> AuthenticatedUser {
    AuthenticatedUser {
        user_id: Uuid::new_v4(),
        role: role.to_string(),
        jti: Uuid::new_v4().to_string(),
    }
}

// ---------------------------------------------------------------------------
// require_role
// ---------------------------------------------------------------------------

#[test]
fn test_require_role_admin_passes() {
    let user = make_user("Admin");
    assert!(require_role(&user, "Admin").is_ok());
}

#[test]
fn test_require_role_admin_fails_for_shopper() {
    let user = make_user("Shopper");
    assert!(require_role(&user, "Admin").is_err());
}

#[test]
fn test_require_role_reviewer_passes() {
    let user = make_user("Reviewer");
    assert!(require_role(&user, "Reviewer").is_ok());
}

#[test]
fn test_require_role_case_sensitive() {
    let user = make_user("admin");
    assert!(require_role(&user, "Admin").is_err());
}

// ---------------------------------------------------------------------------
// require_any_role
// ---------------------------------------------------------------------------

#[test]
fn test_require_any_role_admin_in_list() {
    let user = make_user("Admin");
    assert!(require_any_role(&user, &["Admin", "Reviewer"]).is_ok());
}

#[test]
fn test_require_any_role_reviewer_in_list() {
    let user = make_user("Reviewer");
    assert!(require_any_role(&user, &["Admin", "Reviewer"]).is_ok());
}

#[test]
fn test_require_any_role_shopper_not_in_list() {
    let user = make_user("Shopper");
    assert!(require_any_role(&user, &["Admin", "Reviewer"]).is_err());
}

#[test]
fn test_require_any_role_empty_list() {
    let user = make_user("Admin");
    assert!(require_any_role(&user, &[]).is_err());
}

// ---------------------------------------------------------------------------
// require_owner_or_admin
// ---------------------------------------------------------------------------

#[test]
fn test_owner_access_allowed() {
    let user = make_user("Shopper");
    assert!(require_owner_or_admin(&user, user.user_id).is_ok());
}

#[test]
fn test_admin_access_to_others_resource() {
    let user = make_user("Admin");
    let other_id = Uuid::new_v4();
    assert!(require_owner_or_admin(&user, other_id).is_ok());
}

#[test]
fn test_non_owner_non_admin_denied() {
    let user = make_user("Shopper");
    let other_id = Uuid::new_v4();
    assert!(require_owner_or_admin(&user, other_id).is_err());
}

#[test]
fn test_reviewer_not_admin_denied_for_others() {
    let user = make_user("Reviewer");
    let other_id = Uuid::new_v4();
    assert!(require_owner_or_admin(&user, other_id).is_err());
}
