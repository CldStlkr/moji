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
        <div class="player-scores">
            <h3>"Player Scores"</h3>
            <div class="scores-list">
                {sorted_players.into_iter().enumerate().map(|(index, player)| {
                    let is_current = player.id == current_player_id.get();
                    let is_leader = show_leader_badge && player.id == leader_id;
                    let rank = index + 1;

                    view! {
                        <div class="score-item" class:current-player=is_current>
                            <div class="rank">
                                {rank}
                            </div>
                            <div class="player-info">
                                <span class="player-name">{player.name}</span>
                                <div class="badges">
                                    <Show when=move || is_leader>
                                        <span class="leader-badge" title="Lobby Leader">"👑"</span>
                                    </Show>
                                    <Show when=move || is_current>
                                        <span class="you-badge">"(You)"</span>
                                    </Show>
                                </div>
                            </div>
                            <div class="score">
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
        <div class="compact-player-scores">
            <h4>"Scores"</h4>
            <div class="compact-scores-list">
                {sorted_players.into_iter().map(|player| {
                    let is_current = player.id == current_player_id.get();

                    view! {
                        <div class="compact-score-item" class:current-player=is_current>
                            <span class="player-name">{player.name}</span>
                            <span class="score">{player.score}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
