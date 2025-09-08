// Component for displaying player scores
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
      <div class="p-4 mt-6 bg-gray-50 rounded-lg">
        <h3 class="pb-2 mb-4 text-xl font-semibold text-blue-600 border-b border-gray-200">
          "Player Scores"
        </h3>
        <div class="space-y-2">
          {sorted_players
            .into_iter()
            .enumerate()
            .map(|(index, player)| {
              let is_current = player.id == current_player_id.get();
              let is_leader = show_leader_badge && player.id == leader_id;
              let rank = index + 1;
              view! {
                <div class=format!(
                  "flex items-center gap-4 p-3 rounded-lg border-b border-gray-200 last:border-b-0 {}",
                  if is_current { "bg-blue-50 font-semibold" } else { "bg-white" },
                )>
                  <div class="flex flex-shrink-0 justify-center items-center w-8 h-8 text-sm font-bold text-white bg-blue-500 rounded-full">
                    {rank}
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="flex gap-2 items-center">
                      <span class="font-medium text-gray-900 truncate">{player.name}</span>
                      <div class="flex gap-1 items-center">
                        <Show when=move || is_leader>
                          <span class="text-lg" title="Lobby Leader">
                            "👑"
                          </span>
                        </Show>
                        <Show when=move || is_current>
                          <span class="py-1 px-2 text-xs font-medium text-blue-800 bg-blue-100 rounded-full">
                            "You"
                          </span>
                        </Show>
                      </div>
                    </div>
                  </div>
                  <div class="flex-shrink-0 text-xl font-bold text-blue-600">{player.score}</div>
                </div>
              }
            })
            .collect_view()}
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
      <div class="p-4 mb-4 bg-gray-50 rounded-lg">
        <h4 class="pb-2 mb-3 text-lg font-semibold text-blue-600 border-b border-gray-200">
          "Scores"
        </h4>
        <div class="space-y-1">
          {sorted_players
            .into_iter()
            .map(|player| {
              let is_current = player.id == current_player_id.get();
              view! {
                <div class=format!(
                  "flex justify-between items-center px-3 py-2 rounded bg-white transition-colors hover:bg-gray-50 {}",
                  if is_current {
                    "bg-blue-50 font-semibold border-l-4 border-blue-500"
                  } else {
                    ""
                  },
                )>
                  <span class="font-medium text-gray-900">{player.name}:</span>
                  <span class="ml-2 font-semibold text-blue-600">{player.score}</span>
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
    }
}
