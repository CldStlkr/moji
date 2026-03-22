use leptos::prelude::*;
use crate::context::{GameContext, InGameContext};
use shared::GameMode;

#[component]
pub fn GameOver() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let in_game_context = use_context::<InGameContext>().expect("InGameContext missing");
    
    let lobby_info = game_context.lobby_info;
    let player_id = game_context.player_id;
    let is_leader = game_context.is_leader;
    let on_return_to_lobby = in_game_context.on_return_to_lobby;
    let on_exit = in_game_context.on_exit_game;

    let players = Signal::derive(move || lobby_info.get().map(|i| i.players).unwrap_or_default());
    let mode = Signal::derive(move || lobby_info.get().map(|i| i.settings.mode).unwrap_or_default());

    // Determine winner(s)
    let winner = Signal::derive(move || {
        let players_list = players.get();
        match mode.get() {
            GameMode::Deathmatch => {
                let mut p = players_list.clone();
                p.sort_by(|a, b| b.score.cmp(&a.score));
                p.first().cloned()
            },
            GameMode::Duel => {
                let mut active: Vec<_> = players_list.iter().filter(|p| !p.is_eliminated).cloned().collect();
                if active.is_empty() {
                     let mut p = players_list.clone();
                     p.sort_by(|a, b| b.score.cmp(&a.score));
                     p.first().cloned()
                } else {
                    active.sort_by(|a, b| b.score.cmp(&a.score));
                    active.first().cloned()
                }
            },
            GameMode::Zen => None,
        }
    });

    let is_winner = Signal::derive(move || {
        winner.get().map(|w| w.id == player_id.get()).unwrap_or(false)
    });

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 animate-fade-in backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all scale-100 animate-scale-in text-center border-2 border-gray-100 dark:border-gray-700">

                <div class="mb-6">
                    <span class="text-6xl mb-4 block">
                        {move || if is_winner.get() { "🏆" } else { "💀" }}
                    </span>
                    <h2 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-500 to-purple-600 dark:from-blue-400 dark:to-purple-500">
                        {move || if is_winner.get() { "VICTORY!" } else { "GAME OVER" }}
                    </h2>
                </div>

                <div class="mb-8 space-y-2">
                    <p class="text-gray-600 dark:text-gray-300">
                        {move || match mode.get() {
                            GameMode::Deathmatch => "Target score reached!",
                            GameMode::Duel => "Last player standing!",
                            GameMode::Zen => "Session Ended!",
                        }}
                    </p>

                    <div class="py-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                        <p class="text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold mb-1">
                            "Winner"
                        </p>
                        <p class="text-2xl font-bold text-gray-900 dark:text-white">
                            {move || winner.get().map(|p| p.name).unwrap_or_else(|| "Unknown".to_string())}
                        </p>
                    </div>
                </div>

                <div class="space-y-3">
                    // Restart Lobby Button (Leader Only, Disabled for others)
                    <button
                        on:click=move |_| {
                            if is_leader.get() {
                                on_return_to_lobby.run(());
                            }
                        }
                        disabled=move || !is_leader.get()
                        class=move || {
                            let base_class = "w-full py-3 px-6 rounded-lg font-semibold shadow-lg transition-all transform";
                            if is_leader.get() {
                                format!("{} bg-blue-600 hover:bg-blue-700 text-white hover:shadow-xl hover:-translate-y-0.5 active:translate-y-0", base_class)
                            } else {
                                format!("{} bg-gray-300 dark:bg-gray-700 text-gray-500 cursor-not-allowed opacity-75", base_class)
                            }
                        }
                        title=move || if is_leader.get() { "Return everyone to lobby" } else { "Waiting for leader to restart" }
                    >
                        {move || if is_leader.get() { "Restart Lobby" } else { "Waiting for Leader..." }}
                    </button>

                    // Exit Game Button (For Everyone)
                    <button
                        on:click=move |_| on_exit.run(())
                        class="w-full py-3 px-6 bg-red-500 hover:bg-red-600 text-white rounded-lg font-semibold shadow-md hover:shadow-lg transition-colors border border-red-600 dark:border-red-500"
                    >
                        "Exit Game"
                    </button>
                </div>

            </div>
        </div>
    }
}
