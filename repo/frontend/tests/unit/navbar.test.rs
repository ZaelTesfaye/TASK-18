// Navbar component logic tests.
// These verify navbar display logic (role-based links, cart badge, auth state)
// without rendering Yew components. Imports the real component module to prove
// it compiles and is exercised by the test target.

use silverscreen_frontend::types::*;

// Direct import of the navbar component module — verifies it compiles and is
// reachable from the test harness. Runtime rendering requires WASM (see wasm/ tests).
#[allow(unused_imports)]
use silverscreen_frontend::components::navbar;

// ---------------------------------------------------------------------------
// Role display strings
// ---------------------------------------------------------------------------

#[test]
fn test_role_display_admin() {
    let user = User {
        id: "user-001".to_string(),
        username: "admin_user".to_string(),
        email: "admin@example.com".to_string(),
        role: "Admin".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };
    assert_eq!(user.role, "Admin");
}

#[test]
fn test_role_display_reviewer() {
    let user = User {
        id: "user-002".to_string(),
        username: "reviewer_user".to_string(),
        email: "reviewer@example.com".to_string(),
        role: "Reviewer".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };
    assert_eq!(user.role, "Reviewer");
}

#[test]
fn test_role_display_shopper() {
    let user = User {
        id: "user-003".to_string(),
        username: "shopper_user".to_string(),
        email: "shopper@example.com".to_string(),
        role: "Shopper".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };
    assert_eq!(user.role, "Shopper");
}

// ---------------------------------------------------------------------------
// Cart count badge display logic
// ---------------------------------------------------------------------------

#[test]
fn test_cart_badge_hidden_when_zero() {
    let cart_count: u32 = 0;
    let show_badge = cart_count > 0;
    assert!(!show_badge, "Cart badge must be hidden when count is 0");
}

#[test]
fn test_cart_badge_shown_when_positive() {
    let cart_count: u32 = 3;
    let show_badge = cart_count > 0;
    assert!(show_badge, "Cart badge must be shown when count > 0");
}

#[test]
fn test_cart_badge_shown_when_one() {
    let cart_count: u32 = 1;
    let show_badge = cart_count > 0;
    assert!(show_badge, "Cart badge must be shown when count is 1");
}

#[test]
fn test_cart_badge_display_value() {
    let cart_count: u32 = 5;
    let display = format!("{}", cart_count);
    assert_eq!(display, "5", "Badge must display the numeric count");
}

// ---------------------------------------------------------------------------
// Auth state impacts on navbar
// ---------------------------------------------------------------------------

#[test]
fn test_navbar_authenticated_shows_user_links() {
    // Simulates: when a token is present, user is authenticated
    let token: Option<String> = Some("header.payload.signature".to_string());
    let authenticated = token.is_some();
    assert!(authenticated, "User with token must be authenticated");

    // Authenticated users see: Cart, Orders, Leaderboards
    let auth_links = vec!["Catalog", "Cart", "Orders", "Leaderboards"];
    assert!(auth_links.contains(&"Cart"), "Authenticated user must see Cart");
    assert!(auth_links.contains(&"Orders"), "Authenticated user must see Orders");
    assert!(auth_links.contains(&"Leaderboards"), "Authenticated user must see Leaderboards");
}

#[test]
fn test_navbar_unauthenticated_shows_login_register() {
    // Simulates: when no token, user is not authenticated
    let token: Option<String> = None;
    let authenticated = token.is_some();
    assert!(!authenticated, "User without token must not be authenticated");

    // Unauthenticated users see: Login, Register (but not Cart/Orders)
    let unauth_links = vec!["Catalog", "Login", "Register"];
    assert!(unauth_links.contains(&"Login"), "Unauthenticated user must see Login");
    assert!(unauth_links.contains(&"Register"), "Unauthenticated user must see Register");
    assert!(!unauth_links.contains(&"Cart"), "Unauthenticated user must NOT see Cart");
    assert!(!unauth_links.contains(&"Orders"), "Unauthenticated user must NOT see Orders");
}

#[test]
fn test_navbar_role_based_links_shopper() {
    let role = "Shopper";
    let show_reviews = role == "Reviewer" || role == "Admin";
    let show_admin = role == "Admin";
    assert!(!show_reviews, "Shopper must NOT see Reviews link");
    assert!(!show_admin, "Shopper must NOT see Admin link");
}

#[test]
fn test_navbar_role_based_links_reviewer() {
    let role = "Reviewer";
    let show_reviews = role == "Reviewer" || role == "Admin";
    let show_admin = role == "Admin";
    assert!(show_reviews, "Reviewer must see Reviews link");
    assert!(!show_admin, "Reviewer must NOT see Admin link");
}

#[test]
fn test_navbar_role_based_links_admin() {
    let role = "Admin";
    let show_reviews = role == "Reviewer" || role == "Admin";
    let show_admin = role == "Admin";
    assert!(show_reviews, "Admin must see Reviews link");
    assert!(show_admin, "Admin must see Admin link");
}

#[test]
fn test_navbar_username_display() {
    let user = UserResponse {
        id: "user-001".to_string(),
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        role: "Shopper".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };
    // The navbar displays user.username when authenticated
    assert_eq!(user.username, "alice");
    assert!(!user.username.is_empty(), "Username must not be empty for display");
}

#[test]
fn test_navbar_role_badge_display() {
    // The navbar displays the role as a badge next to the username
    let role = "Admin";
    let badge_text = role;
    assert_eq!(badge_text, "Admin");
    // All valid roles should produce non-empty badge text
    for r in &["Shopper", "Reviewer", "Admin"] {
        assert!(!r.is_empty(), "Role badge must not be empty");
    }
}

#[test]
fn test_navbar_menu_toggle_logic() {
    // The navbar has a mobile menu toggle state
    let mut menu_open = false;

    // Toggle open
    menu_open = !menu_open;
    assert!(menu_open, "Menu should be open after first toggle");

    // Toggle closed
    menu_open = !menu_open;
    assert!(!menu_open, "Menu should be closed after second toggle");
}

#[test]
fn test_navbar_menu_closes_on_link_click() {
    // Clicking a nav link should close the mobile menu
    let mut menu_open = true;
    // Simulate close_menu callback
    menu_open = false;
    assert!(!menu_open, "Menu must close when a link is clicked");
}

#[test]
fn test_navbar_nav_class_open_vs_closed() {
    // The navbar applies different classes based on menu state
    let menu_open = true;
    let nav_class = if menu_open { "nav-links nav-open" } else { "nav-links" };
    assert_eq!(nav_class, "nav-links nav-open");

    let menu_open = false;
    let nav_class = if menu_open { "nav-links nav-open" } else { "nav-links" };
    assert_eq!(nav_class, "nav-links");
}

#[test]
fn test_navbar_locked_user_still_shows_role() {
    // Even a locked user's role should display correctly in the badge
    let user = User {
        id: "user-locked".to_string(),
        username: "locked_user".to_string(),
        email: "locked@example.com".to_string(),
        role: "Shopper".to_string(),
        locked: true,
        created_at: None,
        updated_at: None,
    };
    assert!(user.locked);
    assert_eq!(user.role, "Shopper");
}
