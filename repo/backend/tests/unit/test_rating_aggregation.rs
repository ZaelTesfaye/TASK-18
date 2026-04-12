// Rating aggregation logic unit tests.
//
// The actual aggregate computation runs in PostgreSQL (AVG + ROUND), but we
// test the deterministic tie-breaking and formula logic here as specified:
//   aggregate = avg(per_rating_averages), precision 2 decimal places
//   tie-break: higher total_ratings wins, then most recent last_rating_at

/// Simulates the per-rating average: average of dimension scores for one rating.
fn per_rating_average(scores: &[u32]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let sum: f64 = scores.iter().map(|&s| s as f64).sum();
    sum / scores.len() as f64
}

/// Simulates the product aggregate: average of all per-rating averages, rounded to 2 decimals.
fn product_aggregate(rating_averages: &[f64]) -> f64 {
    if rating_averages.is_empty() {
        return 0.0;
    }
    let sum: f64 = rating_averages.iter().sum();
    let avg = sum / rating_averages.len() as f64;
    (avg * 100.0).round() / 100.0
}

#[test]
fn test_single_rating_single_dimension() {
    let avg = per_rating_average(&[8]);
    assert_eq!(avg, 8.0);
    assert_eq!(product_aggregate(&[avg]), 8.0);
}

#[test]
fn test_single_rating_multi_dimension() {
    // Plot=8, Acting=6, Visuals=10  => per-rating avg = 8.0
    let avg = per_rating_average(&[8, 6, 10]);
    assert_eq!(avg, 8.0);
    assert_eq!(product_aggregate(&[avg]), 8.0);
}

#[test]
fn test_multiple_ratings() {
    // Rating 1: [8, 6, 10] => 8.0
    // Rating 2: [5, 7, 3]  => 5.0
    // Aggregate: (8.0 + 5.0) / 2 = 6.5
    let avg1 = per_rating_average(&[8, 6, 10]);
    let avg2 = per_rating_average(&[5, 7, 3]);
    assert_eq!(product_aggregate(&[avg1, avg2]), 6.5);
}

#[test]
fn test_aggregate_rounding() {
    // Rating 1: [7, 8, 9] => 8.0
    // Rating 2: [6, 5, 4] => 5.0
    // Rating 3: [3, 3, 3] => 3.0
    // Aggregate: (8 + 5 + 3) / 3 = 5.333... => 5.33
    let avg1 = per_rating_average(&[7, 8, 9]);
    let avg2 = per_rating_average(&[6, 5, 4]);
    let avg3 = per_rating_average(&[3, 3, 3]);
    assert_eq!(product_aggregate(&[avg1, avg2, avg3]), 5.33);
}

#[test]
fn test_aggregate_rounding_up() {
    // Rating 1: [10, 10, 10] => 10.0
    // Rating 2: [7, 7, 7]   => 7.0
    // Rating 3: [8, 8, 8]   => 8.0
    // Aggregate: (10 + 7 + 8) / 3 = 8.333... => 8.33
    let avg1 = per_rating_average(&[10, 10, 10]);
    let avg2 = per_rating_average(&[7, 7, 7]);
    let avg3 = per_rating_average(&[8, 8, 8]);
    assert_eq!(product_aggregate(&[avg1, avg2, avg3]), 8.33);
}

#[test]
fn test_dimension_scores_boundary() {
    // Minimum: all 1s.
    assert_eq!(per_rating_average(&[1, 1, 1]), 1.0);
    // Maximum: all 10s.
    assert_eq!(per_rating_average(&[10, 10, 10]), 10.0);
}

// ---------------------------------------------------------------------------
// Leaderboard tie-breaking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LeaderboardEntry {
    product_id: &'static str,
    average_score: f64,
    total_ratings: i32,
    last_activity: i64, // Unix timestamp
}

fn sort_leaderboard(entries: &mut Vec<LeaderboardEntry>) {
    entries.sort_by(|a, b| {
        // Primary: higher average score.
        b.average_score
            .partial_cmp(&a.average_score)
            .unwrap()
            // Tie-break 1: higher total rating count.
            .then(b.total_ratings.cmp(&a.total_ratings))
            // Tie-break 2: most recent activity.
            .then(b.last_activity.cmp(&a.last_activity))
    });
}

#[test]
fn test_leaderboard_basic_ordering() {
    let mut entries = vec![
        LeaderboardEntry {
            product_id: "C",
            average_score: 7.0,
            total_ratings: 10,
            last_activity: 100,
        },
        LeaderboardEntry {
            product_id: "A",
            average_score: 9.0,
            total_ratings: 5,
            last_activity: 100,
        },
        LeaderboardEntry {
            product_id: "B",
            average_score: 8.0,
            total_ratings: 20,
            last_activity: 100,
        },
    ];
    sort_leaderboard(&mut entries);
    assert_eq!(entries[0].product_id, "A");
    assert_eq!(entries[1].product_id, "B");
    assert_eq!(entries[2].product_id, "C");
}

#[test]
fn test_leaderboard_tiebreak_by_count() {
    let mut entries = vec![
        LeaderboardEntry {
            product_id: "A",
            average_score: 8.0,
            total_ratings: 5,
            last_activity: 100,
        },
        LeaderboardEntry {
            product_id: "B",
            average_score: 8.0,
            total_ratings: 15,
            last_activity: 100,
        },
    ];
    sort_leaderboard(&mut entries);
    assert_eq!(entries[0].product_id, "B"); // Higher count wins.
}

#[test]
fn test_leaderboard_tiebreak_by_recency() {
    let mut entries = vec![
        LeaderboardEntry {
            product_id: "A",
            average_score: 8.0,
            total_ratings: 10,
            last_activity: 100,
        },
        LeaderboardEntry {
            product_id: "B",
            average_score: 8.0,
            total_ratings: 10,
            last_activity: 200,
        },
    ];
    sort_leaderboard(&mut entries);
    assert_eq!(entries[0].product_id, "B"); // More recent wins.
}

#[test]
fn test_leaderboard_three_way_tie() {
    let mut entries = vec![
        LeaderboardEntry {
            product_id: "A",
            average_score: 7.5,
            total_ratings: 10,
            last_activity: 100,
        },
        LeaderboardEntry {
            product_id: "B",
            average_score: 7.5,
            total_ratings: 10,
            last_activity: 300,
        },
        LeaderboardEntry {
            product_id: "C",
            average_score: 7.5,
            total_ratings: 10,
            last_activity: 200,
        },
    ];
    sort_leaderboard(&mut entries);
    assert_eq!(entries[0].product_id, "B"); // Most recent.
    assert_eq!(entries[1].product_id, "C");
    assert_eq!(entries[2].product_id, "A");
}
