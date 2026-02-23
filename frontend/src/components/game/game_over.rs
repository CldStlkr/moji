use leptos::prelude::*;
use shared::{PlayerData, GameMode, PlayerId};

#[component]
pub fn GameOver(
    players: Vec<PlayerData>,
    mode: GameMode,
    current_player_id: PlayerId,
    #[prop(into)] is_leader: Signal<bool>,
    #[prop(into)] on_return_to_lobby: Callback<()>,
    #[prop(into)] on_exit: Callback<()>,
) -> impl IntoView
{
    Effect::new(move |_| { let _ = is_leader.get(); });

    // Determine winner(s)
    let winner = match mode {
        GameMode::Deathmatch => {
            // Highest score
            let mut p = players.clone();
            p.sort_by(|a, b| b.score.cmp(&a.score));
            p.first().cloned()
        },
        GameMode::Duel => {
            // Last one standing (not eliminated)
            // Or if multiple not eliminated (unlikely if logic is correct), highest score
            let mut active: Vec<_> = players.iter().filter(|p| !p.is_eliminated).collect();
            if active.is_empty() {
                 // Everyone died? (Shouldn't happen with correct logic, but maybe last two died same turn?)
                 // Fallback to highest score
                 let mut p = players.clone();
                 p.sort_by(|a, b| b.score.cmp(&a.score));
                 p.first().cloned()
            } else {
                active.sort_by(|a, b| b.score.cmp(&a.score));
                active.first().map(|p| (*p).clone())
            }
        }
    };

    let is_winner = winner.as_ref().map(|w| w.id == current_player_id).unwrap_or(false);

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 animate-fade-in backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all scale-100 animate-scale-in text-center border-2 border-gray-100 dark:border-gray-700">

                <div class="mb-6">
                    <span class="text-6xl mb-4 block">
                        {if is_winner { "🏆" } else { "💀" }}
                    </span>
                    <h2 class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-500 to-purple-600 dark:from-blue-400 dark:to-purple-500">
                        {if is_winner { "VICTORY!" } else { "GAME OVER" }}
                    </h2>
                </div>

                <div class="mb-8 space-y-2">
                    <p class="text-gray-600 dark:text-gray-300">
                        {match mode {
                            GameMode::Deathmatch => "Target score reached!",
                            GameMode::Duel => "Last player standing!",
                        }}
                    </p>

                    <div class="py-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                        <p class="text-sm text-gray-500 dark:text-gray-400 uppercase tracking-wider font-semibold mb-1">
                            "Winner"
                        </p>
                        <p class="text-2xl font-bold text-gray-900 dark:text-white">
                            {winner.map(|p| p.name).unwrap_or_else(|| "Unknown".to_string())}
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
