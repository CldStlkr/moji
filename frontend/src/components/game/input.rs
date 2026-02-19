use leptos::ev;
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn GameInput<F1, F2, F3>(
    word: ReadSignal<String>,
    is_loading: ReadSignal<bool>,
    kanji: ReadSignal<String>,
    input_ref: NodeRef<html::Input>,
    on_input: F1,
    on_submit: F2,
    on_keydown: F3,
) -> impl IntoView
where
    F1: Fn(ev::Event) + 'static + Copy,
    F2: Fn(ev::MouseEvent) + 'static + Copy,
    F3: Fn(ev::KeyboardEvent) + 'static + Copy,
{
    view! {
        <div class="space-y-4">
            <input
                node_ref=input_ref
                type="text"
                value=move || word.get()
                on:input=on_input
                on:keydown=on_keydown
                placeholder="Enter a Japanese word with this kanji"
                disabled=move || is_loading.get()
                class="w-full p-3 text-lg border-2 border-gray-300 dark:border-gray-600 dark:bg-gray-900 dark:text-white rounded focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
            />

            <button
                on:click=on_submit
                disabled=move || {
                    is_loading.get() || word.get().trim().is_empty()
                        || kanji.get().is_empty()
                }
                class="w-full bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed text-white font-semibold py-3 px-5 rounded transition-all duration-200 hover:-translate-y-0.5 active:translate-y-0.5 disabled:transform-none"
            >
                "Submit"
            </button>
        </div>
    }
}
