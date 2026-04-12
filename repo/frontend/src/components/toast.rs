use gloo::timers::callback::Timeout;
use yew::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastLevel {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastData {
    pub id: u32,
    pub message: String,
    pub level: ToastLevel,
}

/// Manages a list of active toasts.
#[derive(Clone, PartialEq)]
pub struct ToastManager {
    pub toasts: UseStateHandle<Vec<ToastData>>,
    next_id: UseStateHandle<u32>,
}

impl ToastManager {
    pub fn add(&self, message: String, level: ToastLevel) {
        let id = *self.next_id;
        self.next_id.set(id + 1);
        let toast = ToastData { id, message, level };
        let mut current = (*self.toasts).clone();
        current.push(toast);
        self.toasts.set(current);
    }

    pub fn success(&self, message: &str) {
        self.add(message.to_string(), ToastLevel::Success);
    }

    pub fn error(&self, message: &str) {
        self.add(message.to_string(), ToastLevel::Error);
    }

    pub fn info(&self, message: &str) {
        self.add(message.to_string(), ToastLevel::Info);
    }

    pub fn remove(&self, id: u32) {
        let current: Vec<ToastData> = (*self.toasts)
            .iter()
            .filter(|t| t.id != id)
            .cloned()
            .collect();
        self.toasts.set(current);
    }
}

#[hook]
pub fn use_toast() -> ToastManager {
    let toasts = use_state(Vec::<ToastData>::new);
    let next_id = use_state(|| 1u32);
    ToastManager { toasts, next_id }
}

#[derive(Properties, PartialEq)]
pub struct ToastContainerProps {
    pub manager: ToastManager,
}

#[function_component(ToastContainer)]
pub fn toast_container(props: &ToastContainerProps) -> Html {
    let toasts = (*props.manager.toasts).clone();
    let manager = props.manager.clone();

    html! {
        <div class="toast-container">
            { for toasts.iter().map(|t| {
                let toast = t.clone();
                let mgr = manager.clone();
                html! {
                    <ToastItem toast={toast} manager={mgr} />
                }
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ToastItemProps {
    toast: ToastData,
    manager: ToastManager,
}

#[function_component(ToastItem)]
fn toast_item(props: &ToastItemProps) -> Html {
    let manager = props.manager.clone();
    let id = props.toast.id;

    // Auto-dismiss after 5 seconds
    {
        let manager = manager.clone();
        use_effect_with(id, move |_| {
            let handle = Rc::new(std::cell::RefCell::new(None::<Timeout>));
            let h = handle.clone();
            let timeout = Timeout::new(5_000, move || {
                manager.remove(id);
                drop(h);
            });
            *handle.borrow_mut() = Some(timeout);
            || ()
        });
    }

    let level_class = match props.toast.level {
        ToastLevel::Success => "toast toast-success",
        ToastLevel::Error => "toast toast-error",
        ToastLevel::Info => "toast toast-info",
    };

    let icon = match props.toast.level {
        ToastLevel::Success => "\u{2713}",
        ToastLevel::Error => "\u{2717}",
        ToastLevel::Info => "\u{2139}",
    };

    let on_close = {
        let manager = manager.clone();
        Callback::from(move |_: MouseEvent| {
            manager.remove(id);
        })
    };

    html! {
        <div class={level_class}>
            <span class="toast-icon">{ icon }</span>
            <span class="toast-message">{ &props.toast.message }</span>
            <button class="toast-close" onclick={on_close}>{ "\u{00D7}" }</button>
        </div>
    }
}
