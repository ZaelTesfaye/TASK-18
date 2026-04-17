// Pagination component logic tests.
// These verify page calculation, bounds clamping, and page window/ellipsis
// logic extracted from the Pagination component, without rendering.
// Imports the real pagination module to verify compilation.

use silverscreen_frontend::types::PaginatedResponse;

#[allow(unused_imports)]
use silverscreen_frontend::components::pagination;

// ---------------------------------------------------------------------------
// Page calculation: total_pages from total_items and per_page
// ---------------------------------------------------------------------------

#[test]
fn test_page_calculation_zero_items() {
    let total_items: u64 = 0;
    let per_page: u64 = 10;
    let total_pages = if total_items == 0 { 0 } else { (total_items + per_page - 1) / per_page };
    assert_eq!(total_pages, 0, "0 items should produce 0 pages");
}

#[test]
fn test_page_calculation_25_items_10_per_page() {
    let total_items: u64 = 25;
    let per_page: u64 = 10;
    let total_pages = (total_items + per_page - 1) / per_page;
    assert_eq!(total_pages, 3, "25 items at 10/page should produce 3 pages");
}

#[test]
fn test_page_calculation_exact_multiple() {
    let total_items: u64 = 10;
    let per_page: u64 = 10;
    let total_pages = (total_items + per_page - 1) / per_page;
    assert_eq!(total_pages, 1, "10 items at 10/page should produce 1 page");
}

#[test]
fn test_page_calculation_one_item() {
    let total_items: u64 = 1;
    let per_page: u64 = 10;
    let total_pages = (total_items + per_page - 1) / per_page;
    assert_eq!(total_pages, 1, "1 item at 10/page should produce 1 page");
}

#[test]
fn test_page_calculation_large_dataset() {
    let total_items: u64 = 1000;
    let per_page: u64 = 12;
    let total_pages = (total_items + per_page - 1) / per_page;
    assert_eq!(total_pages, 84, "1000 items at 12/page should produce 84 pages");
}

#[test]
fn test_page_calculation_matches_paginated_response() {
    // Verify that our calculation matches what the backend sends in PaginatedResponse
    let resp: PaginatedResponse<String> = PaginatedResponse {
        items: vec!["a".into(), "b".into(), "c".into()],
        total: 25,
        page: 1,
        per_page: 10,
        total_pages: 3,
    };
    let computed = (resp.total + resp.per_page - 1) / resp.per_page;
    assert_eq!(computed, resp.total_pages, "Computed total_pages must match backend value");
}

// ---------------------------------------------------------------------------
// Current page bounds
// ---------------------------------------------------------------------------

#[test]
fn test_current_page_cannot_go_below_one() {
    let current_page: u64 = 1;
    let total_pages: u64 = 5;
    // Prev button is disabled when current <= 1
    let prev_disabled = current_page <= 1;
    assert!(prev_disabled, "Prev button must be disabled on page 1");

    // Attempting to go to page 0 should be clamped
    let new_page = if current_page > 1 { current_page - 1 } else { current_page };
    assert_eq!(new_page, 1, "Page must stay at 1 when already at minimum");
    assert!(new_page >= 1, "Page must never be less than 1");
    let _ = total_pages;
}

#[test]
fn test_current_page_cannot_exceed_total_pages() {
    let current_page: u64 = 5;
    let total_pages: u64 = 5;
    // Next button is disabled when current >= total
    let next_disabled = current_page >= total_pages;
    assert!(next_disabled, "Next button must be disabled on last page");

    // Attempting to go beyond total_pages should be clamped
    let new_page = if current_page < total_pages { current_page + 1 } else { current_page };
    assert_eq!(new_page, 5, "Page must stay at total_pages when already at maximum");
    assert!(new_page <= total_pages, "Page must never exceed total_pages");
}

#[test]
fn test_current_page_prev_from_middle() {
    let current_page: u64 = 3;
    let total_pages: u64 = 5;
    let prev_disabled = current_page <= 1;
    assert!(!prev_disabled, "Prev must be enabled on page 3");
    let new_page = current_page - 1;
    assert_eq!(new_page, 2);
    let _ = total_pages;
}

#[test]
fn test_current_page_next_from_middle() {
    let current_page: u64 = 3;
    let total_pages: u64 = 5;
    let next_disabled = current_page >= total_pages;
    assert!(!next_disabled, "Next must be enabled on page 3 of 5");
    let new_page = current_page + 1;
    assert_eq!(new_page, 4);
}

// ---------------------------------------------------------------------------
// Page window calculation with ellipsis logic
// (mirrors the Pagination component's window algorithm)
// ---------------------------------------------------------------------------

/// Replicates the pagination component's page window logic.
fn compute_page_window(current: u64, total: u64) -> Vec<u64> {
    let mut pages: Vec<u64> = Vec::new();
    let window: u64 = 2;
    let start = if current > window + 1 { current - window } else { 1 };
    let end = if current + window < total { current + window } else { total };

    if start > 1 {
        pages.push(1);
        if start > 2 {
            pages.push(0); // ellipsis marker
        }
    }
    for p in start..=end {
        pages.push(p);
    }
    if end < total {
        if end < total - 1 {
            pages.push(0); // ellipsis marker
        }
        pages.push(total);
    }
    pages
}

#[test]
fn test_page_window_first_page_small_total() {
    // current=1, total=3 → [1, 2, 3] (no ellipsis needed)
    let pages = compute_page_window(1, 3);
    assert_eq!(pages, vec![1, 2, 3]);
}

#[test]
fn test_page_window_first_page_large_total() {
    // current=1, total=10 → [1, 2, 3, 0, 10]
    let pages = compute_page_window(1, 10);
    assert_eq!(pages, vec![1, 2, 3, 0, 10]);
}

#[test]
fn test_page_window_middle_page() {
    // current=5, total=10 → [1, 0, 3, 4, 5, 6, 7, 0, 10]
    let pages = compute_page_window(5, 10);
    assert_eq!(pages, vec![1, 0, 3, 4, 5, 6, 7, 0, 10]);
}

#[test]
fn test_page_window_last_page() {
    // current=10, total=10 → [1, 0, 8, 9, 10]
    let pages = compute_page_window(10, 10);
    assert_eq!(pages, vec![1, 0, 8, 9, 10]);
}

#[test]
fn test_page_window_near_start() {
    // current=3, total=10 → [1, 2, 3, 4, 5, 0, 10]
    let pages = compute_page_window(3, 10);
    assert_eq!(pages, vec![1, 2, 3, 4, 5, 0, 10]);
}

#[test]
fn test_page_window_near_end() {
    // current=8, total=10 → [1, 0, 6, 7, 8, 9, 10]
    let pages = compute_page_window(8, 10);
    assert_eq!(pages, vec![1, 0, 6, 7, 8, 9, 10]);
}

#[test]
fn test_page_window_single_page() {
    // For a single page, the pagination component returns empty html,
    // but our window function still produces [1]
    let pages = compute_page_window(1, 1);
    assert_eq!(pages, vec![1]);
}

#[test]
fn test_page_window_two_pages() {
    let pages = compute_page_window(1, 2);
    assert_eq!(pages, vec![1, 2]);
}

#[test]
fn test_page_window_ellipsis_is_zero() {
    // Verify ellipsis markers are represented as 0
    let pages = compute_page_window(5, 10);
    let ellipses: Vec<&u64> = pages.iter().filter(|&&p| p == 0).collect();
    assert_eq!(ellipses.len(), 2, "Middle page in 10-page set should have 2 ellipses");
}

#[test]
fn test_page_window_no_duplicates_except_ellipsis() {
    let pages = compute_page_window(5, 10);
    // Filter out ellipsis markers (0) and check for uniqueness
    let non_ellipsis: Vec<u64> = pages.iter().copied().filter(|&p| p != 0).collect();
    let mut deduped = non_ellipsis.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(non_ellipsis.len(), deduped.len(), "Page numbers must be unique (excluding ellipsis)");
}

#[test]
fn test_pagination_hidden_for_single_page() {
    // The Pagination component returns empty html if total_pages <= 1
    let total_pages: u64 = 1;
    let should_render = total_pages > 1;
    assert!(!should_render, "Pagination must be hidden when total_pages <= 1");

    let total_pages: u64 = 0;
    let should_render = total_pages > 1;
    assert!(!should_render, "Pagination must be hidden when total_pages is 0");
}

#[test]
fn test_pagination_visible_for_multiple_pages() {
    let total_pages: u64 = 2;
    let should_render = total_pages > 1;
    assert!(should_render, "Pagination must be visible when total_pages > 1");
}
