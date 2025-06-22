// lobby/player_scores.rs - Component for displaying player scores
use leptos::prelude::*;
use shared::{PlayerData, PlayerId};

#[component]
pub fn PlayerScoresComponent(
    players: Vec<PlayerData>,
    current_player_id: ReadSignal<PlayerId>,
    leader_id: PlayerId,
    show_leader_badge: bool,
) -> impl IntoView {
    // Sort players by score (highest first), then by name for ties
    let mut sorted_players = players;
    sorted_players.sort_by(|a, b| b.score.cmp(&a.score).then(a.name.cmp(&b.name)));

    view! {
        <div class="mt-6 p-4 bg-gray-50 rounded-lg">
            <h3 class="text-xl font-semibold text-blue-600 mb-4 border-b border-gray-200 pb-2">"Player Scores"</h3>
            <div class="space-y-2">
                {sorted_players.into_iter().enumerate().map(|(index, player)| {
                    let is_current = player.id == current_player_id.get();
                    let is_leader = show_leader_badge && player.id == leader_id;
                    let rank = index + 1;
                    view! {
                        <div class=format!(
                            "flex items-center gap-4 p-3 rounded-lg border-b border-gray-200 last:border-b-0 {}",
                            if is_current { "bg-blue-50 font-semibold" } else { "bg-white" }
                        )>
                            <div class="flex-shrink-0 w-8 h-8 bg-blue-500 text-white rounded-full flex items-center justify-center text-sm font-bold">
                                {rank}
                            </div>
                            <div class="flex-1 min-w-0">
                                <div class="flex items-center gap-2">
                                    <span class="font-medium text-gray-900 truncate">{player.name}</span>
                                    <div class="flex items-center gap-1">
                                        <Show when=move || is_leader>
                                            <span class="text-lg" title="Lobby Leader">"👑"</span>
                                        </Show>
                                        <Show when=move || is_current>
                                            <span class="text-xs bg-blue-100 text-blue-800 px-2 py-1 rounded-full font-medium">"You"</span>
                                        </Show>
                                    </div>
                                </div>
                            </div>
                            <div class="flex-shrink-0 text-xl font-bold text-blue-600">
                                {player.score}
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn CompactPlayerScoresComponent(
    players: Vec<PlayerData>,
    current_player_id: ReadSignal<PlayerId>,
) -> impl IntoView {
    // Sort players by score (highest first)
    let mut sorted_players = players;
    sorted_players.sort_by(|a, b| b.score.cmp(&a.score));

    view! {
        <div class="bg-gray-50 rounded-lg p-4 mb-4">
            <h4 class="text-lg font-semibold text-blue-600 mb-3 pb-2 border-b border-gray-200">
                "Scores"
            </h4>
            <div class="space-y-1">
                {sorted_players.into_iter().map(|player| {
                    let is_current = player.id == current_player_id.get();
                    view! {
                        <div class=format!(
                            "flex justify-between items-center px-3 py-2 rounded bg-white transition-colors hover:bg-gray-50 {}",
                            if is_current { "bg-blue-50 font-semibold border-l-4 border-blue-500" } else { "" }
                        )>
                            <span class="font-medium text-gray-900">{player.name}:</span>
                            <span class="font-semibold text-blue-600 ml-2">{player.score}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
