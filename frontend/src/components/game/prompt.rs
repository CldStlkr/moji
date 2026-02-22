use leptos::prelude::*;

#[component]
pub fn PromptDisplay(
    prompt: ReadSignal<String>,
    is_loading: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <div
            class="flex justify-center items-center bg-gray-100 dark:bg-gray-700 rounded-lg border-2 border-gray-300 dark:border-gray-600 transition-colors"
            style="height: 320px;"
        >
            <Show
                when=move || is_loading.get()
                fallback=move || {
                    view! {
                        <div class="text-9xl leading-none text-gray-800 dark:text-gray-100 kanji-font select-none">
                            {move || prompt.get()}
                        </div>
                    }
                }
            >
                <div class="text-lg text-gray-500 dark:text-gray-400">"Loading..."</div>
            </Show>
        </div>
    }
}
