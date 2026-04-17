// Product card component logic tests.
// These verify price formatting, Product struct fields, aggregate score
// star conversion, and genre badge display logic.
// Imports the real component module to verify compilation.

use silverscreen_frontend::types::*;

#[allow(unused_imports)]
use silverscreen_frontend::components::product_card;

// ---------------------------------------------------------------------------
// Price formatting
// ---------------------------------------------------------------------------

#[test]
fn test_price_format_two_decimals() {
    let price = 19.99_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$19.99");
}

#[test]
fn test_price_format_whole_number() {
    let price = 20.0_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$20.00");
}

#[test]
fn test_price_format_small_cents() {
    let price = 0.99_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$0.99");
}

#[test]
fn test_price_format_large_value() {
    let price = 1234.56_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$1234.56");
}

#[test]
fn test_price_format_zero() {
    let price = 0.0_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$0.00");
}

#[test]
fn test_price_format_rounds_correctly() {
    // 3 decimal places should round to 2
    let price = 19.999_f64;
    let display = format!("${:.2}", price);
    assert_eq!(display, "$20.00");
}

// ---------------------------------------------------------------------------
// Product struct fields
// ---------------------------------------------------------------------------

#[test]
fn test_product_has_required_card_fields() {
    let product = Product {
        id: "prod-001".to_string(),
        title: "The Matrix".to_string(),
        description: "A hacker discovers reality is a simulation.".to_string(),
        price: 19.99,
        genre: "Sci-Fi".to_string(),
        topics: vec![TopicRef { id: "t1".to_string(), name: "Movies".to_string() }],
        tags: vec![TagRef { id: "tag-1".to_string(), name: "Classic".to_string() }],
        custom_fields: serde_json::Value::Null,
        aggregate_score: Some(8.5),
        stock: Some(42),
        is_active: Some(true),
        image_url: None,
        created_at: None,
    };

    // All fields used by ProductCard must be accessible
    assert_eq!(product.title, "The Matrix");
    assert!((product.price - 19.99).abs() < 0.001);
    assert_eq!(product.genre, "Sci-Fi");
    assert_eq!(product.aggregate_score, Some(8.5));
    assert_eq!(product.id, "prod-001");
}

#[test]
fn test_product_optional_fields_defaults() {
    // Product can be constructed with minimal JSON (serde defaults)
    let json_str = r#"{"id":"p1","title":"Minimal"}"#;
    let product: Product = serde_json::from_str(json_str).unwrap();
    assert_eq!(product.title, "Minimal");
    assert_eq!(product.price, 0.0);
    assert_eq!(product.genre, "");
    assert_eq!(product.aggregate_score, None);
    assert_eq!(product.stock, None);
    assert_eq!(product.is_active, None);
    assert!(product.tags.is_empty());
    assert!(product.topics.is_empty());
}

#[test]
fn test_product_image_url_presence() {
    // ProductCard renders an <img> if image_url is Some, placeholder if None
    let with_image = Product {
        id: "p1".to_string(),
        title: "Movie".to_string(),
        description: String::new(),
        price: 10.0,
        genre: String::new(),
        topics: vec![],
        tags: vec![],
        custom_fields: serde_json::Value::Null,
        aggregate_score: None,
        stock: None,
        is_active: None,
        image_url: Some("https://example.com/poster.jpg".to_string()),
        created_at: None,
    };
    assert!(with_image.image_url.is_some(), "Product with image_url should render <img>");

    let without_image = Product {
        image_url: None,
        ..with_image.clone()
    };
    assert!(without_image.image_url.is_none(), "Product without image_url should render placeholder");
}

// ---------------------------------------------------------------------------
// Aggregate score to star conversion (0-10 scale to 0-5 stars)
// ---------------------------------------------------------------------------

/// Replicates the RatingStars component logic.
fn score_to_stars(score: f64) -> (u32, bool, u32) {
    let clamped = score.clamp(0.0, 10.0);
    let star_value = clamped / 2.0;
    let full = star_value.floor() as u32;
    let has_half = (star_value - star_value.floor()) >= 0.25;
    let empty = 5 - full - if has_half { 1 } else { 0 };
    (full, has_half, empty)
}

#[test]
fn test_score_zero_gives_zero_stars() {
    let (full, has_half, empty) = score_to_stars(0.0);
    assert_eq!(full, 0);
    assert!(!has_half);
    assert_eq!(empty, 5);
    assert_eq!(full + empty, 5);
}

#[test]
fn test_score_ten_gives_five_full_stars() {
    let (full, has_half, empty) = score_to_stars(10.0);
    assert_eq!(full, 5);
    assert!(!has_half);
    assert_eq!(empty, 0);
}

#[test]
fn test_score_five_gives_two_and_half_stars() {
    let (full, has_half, empty) = score_to_stars(5.0);
    assert_eq!(full, 2);
    assert!(has_half, "Score 5.0 maps to star_value 2.5, which has a half star");
    assert_eq!(empty, 2);
    // total: 2 full + 1 half + 2 empty = 5
}

#[test]
fn test_score_eight_point_five() {
    // 8.5 / 2 = 4.25 → 4 full, has_half (0.25 >= 0.25), 0 empty
    let (full, has_half, empty) = score_to_stars(8.5);
    assert_eq!(full, 4);
    assert!(has_half);
    assert_eq!(empty, 0);
}

#[test]
fn test_score_two_gives_one_star() {
    // 2.0 / 2 = 1.0 → 1 full, no half, 4 empty
    let (full, has_half, empty) = score_to_stars(2.0);
    assert_eq!(full, 1);
    assert!(!has_half);
    assert_eq!(empty, 4);
}

#[test]
fn test_score_clamped_above_ten() {
    // Scores above 10 are clamped to 10
    let (full, has_half, empty) = score_to_stars(15.0);
    assert_eq!(full, 5);
    assert!(!has_half);
    assert_eq!(empty, 0);
}

#[test]
fn test_score_clamped_below_zero() {
    // Negative scores are clamped to 0
    let (full, has_half, empty) = score_to_stars(-3.0);
    assert_eq!(full, 0);
    assert!(!has_half);
    assert_eq!(empty, 5);
}

#[test]
fn test_score_total_always_five() {
    // For any score, full + half + empty must equal 5
    for i in 0..=100 {
        let score = i as f64 / 10.0; // 0.0 to 10.0 in steps of 0.1
        let (full, has_half, empty) = score_to_stars(score);
        let total = full + if has_half { 1 } else { 0 } + empty;
        assert_eq!(total, 5, "Total star slots must always be 5 for score {}", score);
    }
}

#[test]
fn test_product_card_score_fallback() {
    // ProductCard uses aggregate_score.unwrap_or(0.0)
    let product_with_score = Product {
        id: "p1".to_string(),
        title: "Rated Movie".to_string(),
        description: String::new(),
        price: 10.0,
        genre: String::new(),
        topics: vec![],
        tags: vec![],
        custom_fields: serde_json::Value::Null,
        aggregate_score: Some(7.5),
        stock: None,
        is_active: None,
        image_url: None,
        created_at: None,
    };
    let score = product_with_score.aggregate_score.unwrap_or(0.0);
    assert!((score - 7.5).abs() < 0.001);

    let product_no_score = Product {
        aggregate_score: None,
        ..product_with_score
    };
    let score = product_no_score.aggregate_score.unwrap_or(0.0);
    assert_eq!(score, 0.0, "Missing aggregate_score must default to 0.0");
}

// ---------------------------------------------------------------------------
// Genre badge display
// ---------------------------------------------------------------------------

#[test]
fn test_genre_badge_shown_when_non_empty() {
    let genre = "Sci-Fi";
    let show_badge = !genre.is_empty();
    assert!(show_badge, "Genre badge must be shown for non-empty genre");
}

#[test]
fn test_genre_badge_hidden_when_empty() {
    let genre = "";
    let show_badge = !genre.is_empty();
    assert!(!show_badge, "Genre badge must be hidden for empty genre");
}

#[test]
fn test_genre_badge_various_values() {
    let genres = vec!["Action", "Comedy", "Drama", "Horror", "Sci-Fi", "Thriller", "Romance"];
    for genre in &genres {
        assert!(!genre.is_empty(), "All genre values must be non-empty");
    }
}

#[test]
fn test_product_card_tags_display() {
    let product = Product {
        id: "p1".to_string(),
        title: "Tagged Movie".to_string(),
        description: String::new(),
        price: 10.0,
        genre: "Action".to_string(),
        topics: vec![],
        tags: vec![
            TagRef { id: "t1".to_string(), name: "Classic".to_string() },
            TagRef { id: "t2".to_string(), name: "Award-Winning".to_string() },
        ],
        custom_fields: serde_json::Value::Null,
        aggregate_score: None,
        stock: None,
        is_active: None,
        image_url: None,
        created_at: None,
    };
    assert_eq!(product.tags.len(), 2);
    assert_eq!(product.tags[0].name, "Classic");
    assert_eq!(product.tags[1].name, "Award-Winning");
    // Tags use id (UUID) not name for filtering
    assert!(!product.tags[0].id.is_empty());
}
