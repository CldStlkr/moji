// frontend/src/components/game_over.rs
use crate::{
    api::restart_game,
    error::{get_user_friendly_message, log_error},
};
use leptos::ev;
use leptos::prelude::*;
use shared::{LobbyInfo, PlayerId, RestartGameRequest};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn GameOverComponent<F1, F2>(
    lobby_info: LobbyInfo,
    current_player_id: PlayerId,
    on_leave_game: F1,
    on_return_to_lobby: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static + Copy,
{
    let (is_loading, set_is_loading) = signal(false);
    let (status_message, set_status_message) = signal(String::new());

    let winner_id = lobby_info.winner.clone().unwrap_or_default();
    let is_leader = lobby_info.leader_id == current_player_id;
    let lobby_id = lobby_info.lobby_id.clone();
    let player_id = current_player_id.clone();

    // Find winner's data
    let winner_data = lobby_info
        .players
        .iter()
        .find(|p| p.id == winner_id)
        .cloned();

    // Sort players by score for final standings
    let mut sorted_players = lobby_info.players.clone();
    sorted_players.sort_by(|a, b| b.score.cmp(&a.score));

    let handle_restart = StoredValue::new(move |_: ev::MouseEvent| {
        let lobby_id = lobby_id.clone();
        let player_id = player_id.clone();

        spawn_local(async move {
            set_is_loading.set(true);
            set_status_message.set("Restarting game...".to_string());

            let request = RestartGameRequest { player_id };

            match restart_game(&lobby_id, request).await {
                Ok(_) => {
                    set_status_message.set("Game restarted!".to_string());
                    // The lobby polling will detect the status change
                }
                Err(e) => {
                    log_error("Failed to restart game", &e);
                    set_status_message.set(get_user_friendly_message(&e));
                }
            }

            set_is_loading.set(false);
        });
    });

    let handle_leave = move |_: ev::MouseEvent| {
        on_leave_game();
    };

    view! {
        <div class="max-w-4xl mx-auto my-8 p-8 bg-white rounded-lg shadow-lg">
            // Winner announcement
            <div class="text-center mb-8">
                <h1 class="text-5xl font-bold text-yellow-500 mb-4"> Game Over! </h1>
                {winner_data.map(|winner| view! {
                    <div>
                        <h2 class="text-3xl font-bold text-gray-800 mb-2">
                            "Winner: " <span class="text-blue-600">{winner.name}</span>
                        </h2>
                        <p class="text-xl text-gray-600">
                            "Final Score: " <span class="font-bold">{winner.score}</span>
                        </p>
                    </div>
                })}
            </div>

            // Final Standings
            <div class="mb-8">
                <h3 class="text-2xl font-semibold text-gray-800 mb-4 text-center">Final Standings</h3>
                <div class="max-w-md mx-auto">
                    {sorted_players.into_iter().enumerate().map(|(index, player)| {
                        let is_winner = player.id == winner_id;
                        let is_current = player.id == current_player_id;
                        let place = index + 1;

                        view! {
                            <div class=format!(
                                "flex items-center justify-between p-4 mb-2 rounded-lg {} {}",
                                if is_winner { "bg-yellow-50 border-2 border-yellow-400" }
                                else { "bg-gray-50 border border-gray-200" },
                                if is_current { "ring-2 ring-blue-400" } else { "" }
                            )>
                                <div class="flex items-center gap-3">
                                    <div class=format!(
                                        "w-10 h-10 rounded-full flex items-center justify-center font-bold text-white {}",
                                        match place {
                                            1 => "bg-yellow-500",
                                            2 => "bg-gray-400",
                                            3 => "bg-orange-600",
                                            _ => "bg-gray-600"
                                        }
                                    )>
                                        {place}
                                    </div>
                                    <div>
                                        <span class="font-semibold text-gray-800">
                                            {player.name}
                                            {if is_current { " (You)" } else { "" }}
                                        </span>
                                    </div>
                                </div>
                                <div class="text-xl font-bold text-gray-700">
                                    {player.score} " points"
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Actions
            <div class="flex flex-col gap-4 items-center">
                <Show
                    when=move || is_leader
                    fallback=|| view! {
                        <p class="text-gray-600 italic text-center mb-4">
                            "Waiting for the leader to restart the game..."
                        </p>
                    }
                >
                    <button
                        on:click=handle_restart.get_value()
                        disabled=move || is_loading.get()
                        class="bg-green-500 hover:bg-green-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white font-semibold py-3 px-8 rounded-lg transition-colors text-lg"
                    >
                        "Play Again"
                    </button>
                </Show>

                <button
                    on:click=handle_leave
                    disabled=move || is_loading.get()
                    class="bg-transparent hover:bg-gray-50 text-gray-600 border border-gray-400 font-medium py-2 px-6 rounded transition-colors"
                >
                    "Leave Lobby"
                </button>

                // Status message
                <Show when=move || !status_message.get().is_empty()>
                    <p class="text-gray-600 mt-2">{move || status_message.get()}</p>
                </Show>
            </div>
        </div>
    }
}
