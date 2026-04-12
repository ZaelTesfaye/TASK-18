use yew::prelude::*;

/// Display a rating on a 1-10 scale as 5 stars (each star = 2 points).
#[derive(Properties, PartialEq)]
pub struct RatingStarsProps {
    /// Score from 0.0 to 10.0
    pub score: f64,
    #[prop_or(false)]
    pub show_value: bool,
}

#[function_component(RatingStars)]
pub fn rating_stars(props: &RatingStarsProps) -> Html {
    let score = props.score.clamp(0.0, 10.0);
    // Map 0-10 to 0-5 stars
    let star_value = score / 2.0;
    let full = star_value.floor() as u32;
    let has_half = (star_value - star_value.floor()) >= 0.25;
    let empty = 5 - full - if has_half { 1 } else { 0 };

    html! {
        <span class="rating-stars" title={format!("{:.1}/10", score)}>
            { for (0..full).map(|_| html! { <span class="star star-full">{ "\u{2605}" }</span> }) }
            { if has_half {
                html! { <span class="star star-half">{ "\u{2605}" }</span> }
            } else {
                html! {}
            }}
            { for (0..empty).map(|_| html! { <span class="star star-empty">{ "\u{2606}" }</span> }) }
            { if props.show_value {
                html! { <span class="rating-value">{ format!(" {:.1}", score) }</span> }
            } else {
                html! {}
            }}
        </span>
    }
}
