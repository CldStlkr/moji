use leptos::prelude::*;

#[component]
pub fn PromptDisplay(
    prompt: ReadSignal<String>,
    is_loading: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <div
            class="flex justify-center items-center bg-gray-100 dark:bg-gray-700 rounded-lg border-2 border-gray-300 dark:border-gray-600 transition-colors h-48 sm:h-64 lg:h-80 overflow-hidden p-4"
        >
            <Show
                when=move || is_loading.get()
                fallback=move || {
                    view! {
                        <div class="text-6xl sm:text-8xl lg:text-9xl leading-tight text-gray-800 dark:text-gray-100 kanji-font select-none text-center break-words max-w-full">
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
