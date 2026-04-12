use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoadingProps {
    #[prop_or("Loading...".to_string())]
    pub message: String,
}

#[function_component(Loading)]
pub fn loading(props: &LoadingProps) -> Html {
    html! {
        <div class="loading-container">
            <div class="loading-spinner"></div>
            <p class="loading-text">{ &props.message }</p>
        </div>
    }
}
