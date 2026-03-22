use leptos::prelude::*;
use crate::context::{GameContext, InGameContext};

#[component]
pub fn GameHeader() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let in_game_context = use_context::<InGameContext>().expect("InGameContext missing");
    
    let lobby_info = game_context.lobby_info;
    let player_id = game_context.player_id;
    let on_exit = in_game_context.on_exit_game;

    let content_mode = Signal::derive(move || {
        lobby_info.get().map(|i| i.settings.content_mode).unwrap_or_default()
    });

    let player_name = Signal::derive(move || {
        lobby_info.get()
            .and_then(|info| info.players.into_iter().find(|p| p.id == player_id.get()))
            .map(|p| p.name)
            .unwrap_or_else(|| "Unknown".to_string())
    });

    let score = Signal::derive(move || {
        lobby_info.get()
            .and_then(|info| info.players.into_iter().find(|p| p.id == player_id.get()))
            .map(|p| p.score)
            .unwrap_or_default()
    });

    view! {
        <div class="flex justify-between items-center mb-4 sm:mb-6 flex-wrap gap-2 sm:gap-4">
            <h2 class="text-xl sm:text-2xl font-bold text-gray-800 dark:text-gray-100">{
                move || {
                    match content_mode.get() {
                        shared::ContentMode::Vocab => "Vocab",
                        shared::ContentMode::Kanji => "Kanji",
                    }
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
                    on:click=move |_| on_exit.run(())
                    class="bg-transparent hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 border border-gray-400 dark:border-gray-500 font-medium py-2 px-4 rounded transition-all hover:-translate-y-0.5 active:translate-y-0.5"
                >
                    "Exit Game"
                </button>
            </div>
        </div>
    }
}
