use leptos::prelude::*;
use shared::ContentMode;

#[component]
pub fn GameHeader<F>(
    content_mode: ContentMode,
    player_name: impl Into<Signal<String>> + 'static,
    score: impl Into<Signal<u32>> + 'static,
    on_exit: F,
) -> impl IntoView
where
    F: Fn() + 'static + Copy,
{
    let player_name = player_name.into();
    let score = score.into();
    view! {
        <div class="flex justify-between items-center mb-4 sm:mb-6 flex-wrap gap-2 sm:gap-4">
            <h2 class="text-xl sm:text-2xl font-bold text-gray-800 dark:text-gray-100">{
                move || if content_mode == ContentMode::Vocab {
                    "Vocab"
                } else {
                    "Kanji"
                }
            }</h2>
            <div class="flex items-center gap-4 flex-wrap">
                <div class="bg-blue-50 dark:bg-blue-900/30 px-3 py-1 rounded-full text-sm text-blue-700 dark:text-blue-300 flex items-center whitespace-nowrap">
                    "Player: "
                    <span class="font-semibold ml-1">{move || player_name.get()}</span>
                </div>
                <div class="text-xl font-bold text-blue-500 dark:text-blue-400">
                    "Score: " {move || score.get()}
                </div>
                <button
                    on:click=move |_| on_exit()
                    class="bg-transparent hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 border border-gray-400 dark:border-gray-500 font-medium py-2 px-4 rounded transition-all hover:-translate-y-0.5 active:translate-y-0.5"
                >
                    "Exit Game"
                </button>
            </div>
        </div>
    }
}
