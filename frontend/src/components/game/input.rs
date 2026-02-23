use crate::styled_view;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos_dom::helpers::window_event_listener;
use shared::ContentMode;

styled_view!(game_input_field, "w-full p-3 text-lg border-2 border-gray-300 dark:border-gray-600 dark:bg-gray-900 dark:text-white rounded focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none transition-colors disabled:opacity-60 disabled:cursor-not-allowed");
styled_view!(game_submit_button, disabled: bool,
    "w-full text-white font-semibold py-3 px-5 rounded transition-all duration-200",
    if disabled {
        "bg-gray-400 dark:bg-gray-600 cursor-not-allowed transform-none"
    } else {
        "bg-blue-500 hover:bg-blue-600 hover:-translate-y-0.5 active:translate-y-0.5"
    }
);

#[component]
pub fn GameInput<F1, F2, F3>(
    content_mode: ContentMode,
    word: ReadSignal<String>,
    is_loading: ReadSignal<bool>,
    prompt: ReadSignal<String>,
    input_ref: NodeRef<html::Input>,
    on_input: F1,
    on_submit: F2,
    on_keydown: F3,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView
where
    F1: Fn(ev::Event) + 'static + Copy,
    F2: Fn(ev::MouseEvent) + 'static + Copy,
    F3: Fn(ev::KeyboardEvent) + 'static + Copy,
{

    let mode = StoredValue::new(content_mode);

    window_event_listener(ev::keydown, move |e: ev::KeyboardEvent| {
        let key = e.key();
        if e.meta_key() || e.ctrl_key() | e.alt_key()
            || key == "Tab" || key == "Escape" || key.starts_with('F')
            { return; }

        if let Some(input) = input_ref.get() {
            // Only intercept if the input isn't already focused
            if !input.is_same_node(document()
                .active_element().as_ref()
                .map(|e| e.as_ref()))
            {
                e.prevent_default();
                let _ = input.focus();
            }
        }
    });

    let is_btn_disabled = move || {
        is_loading.get() || word.get().trim().is_empty()
            || prompt.get().is_empty() || disabled.get()
    };

    view! {
        <div class="space-y-4">
            <input
                node_ref=input_ref
                type="text"
                value=move || word.get()
                on:input=on_input
                on:keydown=on_keydown

                placeholder=move || {
                    if mode.get_value() == ContentMode::Vocab {
                        "Enter the reading in hiragana"
                    } else {
                        "Enter a Japanese word with this kanji"
                    }
                }
                disabled=move || is_loading.get() || disabled.get()
                class=game_input_field()
            />

            <button
                on:click=on_submit
                disabled=is_btn_disabled
                class=move || game_submit_button(is_btn_disabled())
            >
                "Submit"
            </button>
        </div>
    }
}
