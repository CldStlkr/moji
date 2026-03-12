use leptos::prelude::*;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub toast_type: ToastType,
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    pub push: Callback<(String, ToastType)>,
}

pub fn provide_toast_context() {
    let toasts = RwSignal::new(Vec::<Toast>::new());
    let counter = RwSignal::new(0u64);

    let push = Callback::new(move |(message, toast_type): (String, ToastType)| {
        let id = counter.get_untracked();
        counter.update(|c| *c += 1);

        toasts.update(|t| t.push(Toast { id, message, toast_type }));

        // Auto-remove after 3 seconds
        set_timeout(
            move || {
                toasts.update(|t| t.retain(|toast| toast.id != id));
            },
            Duration::from_secs(3),
        );
    });

    provide_context(ToastContext { push });

    // Store toasts in a global signal that can be accessed by the container
    provide_context(toasts);
}

pub fn use_toast() -> ToastContext {
    use_context::<ToastContext>().expect("ToastContext not provided")
}

#[component]
pub fn ToastContainer() -> impl IntoView {
    let toasts = use_context::<RwSignal<Vec<Toast>>>().expect("Toast signal not provided");

    view! {
        <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
            <For
                each=move || toasts.get()
                key=|toast| toast.id
                children=move |toast| {
                    let bg_color = match toast.toast_type {
                        ToastType::Success => "bg-green-500",
                        ToastType::Info => "bg-blue-500",
                        ToastType::Warning => "bg-yellow-500",
                        ToastType::Error => "bg-red-500",
                    };

                    view! {
                        <div class=format!(
                            "{} text-white px-4 py-3 rounded-lg shadow-xl flex items-center gap-3 animate-toast-in min-w-[200px] pointer-events-auto",
                            bg_color
                        )>
                            <span class="font-medium">{toast.message}</span>
                            <button 
                                on:click=move |_| {
                                    toasts.update(|t| t.retain(|top_toast| top_toast.id != toast.id));
                                }
                                class="ml-auto hover:bg-white/20 rounded p-0.5 transition-colors"
                            >
                                "✕"
                            </button>
                        </div>
                    }
                }
            />
        </div>
    }
}
