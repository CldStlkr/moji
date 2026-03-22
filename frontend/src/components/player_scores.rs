// Component for displaying player scores
use leptos::prelude::*;
use shared::{PlayerData, PlayerId, GameMode};
use crate::context::GameContext;

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

    let ranked_players: Vec<(usize, PlayerData)> = sorted_players.into_iter().enumerate().map(|(i, p)| (i + 1, p)).collect();

    view! {
        <div class="mt-6 p-4 bg-gray-50 rounded-lg">
            <h3 class="text-xl font-semibold text-blue-600 mb-4 border-b border-gray-200 pb-2">
                "Player Scores"
            </h3>
            <div class="space-y-2">
                <For
                    each=move || ranked_players.clone()
                    key=|(_rank, player)| player.id.clone()
                    children=move |(rank, player)| {
                        let is_current = player.id == current_player_id.get();
                        let is_leader = show_leader_badge && player.id == leader_id;
                        view! {
                            <div class=format!(
                                "flex items-center gap-4 p-3 rounded-lg border-b border-gray-200 last:border-b-0 {}",
                                if is_current { "bg-blue-50 font-semibold" } else { "bg-white" },
                            )>
                                <div class="flex-shrink-0 w-8 h-8 bg-blue-500 text-white rounded-full flex items-center justify-center text-sm font-bold">
                                    {rank}
                                </div>
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2">
                                        <span class="font-medium text-gray-900 truncate">
                                            {player.name.clone()}
                                        </span>
                                        <div class="flex items-center gap-1">
                                            <Show when=move || is_leader>
                                                <span class="text-lg" title="Lobby Leader">
                                                    "👑"
                                                </span>
                                            </Show>
                                            <Show when=move || is_current>
                                                <span class="text-xs bg-blue-100 text-blue-800 px-2 py-1 rounded-full font-medium">
                                                    "You"
                                                </span>
                                            </Show>
                                        </div>
                                    </div>
                                </div>
                                <div class="flex-shrink-0 text-xl font-bold text-blue-600">
                                    {player.score}
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}

#[component]
pub fn CompactPlayerScoresComponent() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    
    let lobby_info = game_context.lobby_info;
    let current_player_id = game_context.player_id;
    let typing_status = game_context.typing_status;

    let sorted_players = Signal::derive(move || {
        let mut p = lobby_info.get().map(|i| i.players).unwrap_or_default();
        p.sort_by(|a, b| b.score.cmp(&a.score));
        p
    });
    
    let game_mode = Signal::derive(move || {
        lobby_info.get().map(|i| i.settings.mode).unwrap_or_default()
    });
    
    let initial_lives = Signal::derive(move || {
        lobby_info.get().map(|i| i.settings.initial_lives.unwrap_or(3)).unwrap_or(3)
    });

    view! {
        <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4 mb-4 transition-colors">
            <h4 class="text-lg font-semibold text-blue-600 dark:text-blue-400 mb-3 pb-2 border-b border-gray-200 dark:border-gray-600 flex justify-between items-center">
                <span>"Scores"</span>
                <Show when=move || game_mode.get() == GameMode::Duel>
                    <span class="text-xs font-normal text-gray-500 dark:text-gray-400 bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded">"Duel Mode"</span>
                </Show>
            </h4>
            <div class="space-y-1">
                <For
                    each=move || sorted_players.get()
                    key=|player| player.id.clone()
                    children=move |player| {
                        let pid = player.id.clone();
                        let sig_pid = pid.clone();
                        let sig_player = player.clone();
                        let player_sig = Signal::derive(move || {
                            sorted_players.get().into_iter().find(|p| p.id == sig_pid).unwrap_or_else(|| sig_player.clone())
                        });

                        view! {
                            <div class=move || {
                                let p = player_sig.get();
                                let is_current = p.id == current_player_id.get();
                                let is_eliminated = p.is_eliminated;
                                format!(
                                    "flex flex-col px-4 py-3 rounded-lg mb-2 shadow-sm transition-all border {} {}",
                                    if is_current {
                                        "bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800"
                                    } else {
                                        "bg-white dark:bg-gray-800 border-gray-100 dark:border-gray-700"
                                    },
                                    if is_eliminated || !p.is_connected { "opacity-60 grayscale bg-gray-100 dark:bg-gray-800" } else { "" }
                                )
                            }>
                                <div class="flex justify-between items-center w-full">
                                    <div class="flex items-center gap-3">
                                        // Rank/Status Indicator
                                        <div class=move || {
                                            let p = player_sig.get();
                                            let mode = game_mode.get();
                                            format!(
                                                "w-2 h-2 rounded-full {}",
                                                if p.is_turn && mode == GameMode::Duel { "bg-green-500 animate-pulse shadow-[0_0_8px_rgba(34,197,94,0.6)]" }
                                                else if p.is_eliminated { "bg-red-500" }
                                                else { "bg-gray-300 dark:bg-gray-600" }
                                            )
                                        }></div>

                                        <div class="flex flex-col min-w-0">
                                            <span class=move || {
                                                let is_current = player_sig.get().id == current_player_id.get();
                                                format!(
                                                    "font-medium text-lg truncate {}",
                                                    if is_current { "text-blue-700 dark:text-blue-300" }
                                                    else { "text-gray-900 dark:text-gray-200" }
                                                )
                                            }>{move || player_sig.get().name.clone()}</span>

                                            <Show when=move || player_sig.get().is_eliminated>
                                                <span class="text-[10px] uppercase tracking-wider font-bold text-red-500">"Eliminated"</span>
                                            </Show>
                                        </div>
                                    </div>

                                    <div class="flex items-center gap-6">
                                         <Show when=move || game_mode.get() == GameMode::Duel && !player_sig.get().is_eliminated>
                                            <div class="flex items-center gap-1">
                                                {move || {
                                                    let p = player_sig.get();
                                                    let current_lives = p.lives.unwrap_or(0);
                                                    let initial_lives_val = initial_lives.get();
                                                    
                                                    (0..initial_lives_val).map(|i| {
                                                        let is_lost = i >= current_lives;
                                                        let is_last = !is_lost && current_lives == 1;
                                                        
                                                        let heart_class = if is_last {
                                                            "text-xl leading-none animate-vibrate inline-block"
                                                        } else if is_lost {
                                                            "text-xl leading-none opacity-40 grayscale-[0.5] inline-block"
                                                        } else {
                                                            "text-xl leading-none inline-block transform hover:scale-125 transition-transform"
                                                        };

                                                        view! {
                                                            <span class=heart_class title=if is_lost { "Lost life" } else { "Life" }>
                                                                {if is_lost { "💔" } else { "❤️" }}
                                                            </span>
                                                        }
                                                    }).collect_view()
                                                }}
                                            </div>
                                         </Show>

                                        <Show when=move || game_mode.get() != GameMode::Duel>
                                            <div class="flex flex-col items-end min-w-[3rem]">
                                                 <span class="text-[10px] text-gray-400 uppercase tracking-wider font-semibold">"Pts"</span>
                                                 <span class="font-bold text-2xl text-blue-600 dark:text-blue-400 leading-none">{move || player_sig.get().score}</span>
                                            </div>
                                        </Show>
                                    </div>
                                </div>

                                <Show when={
                                    let pid_show = pid.clone();
                                    move || typing_status.get().get(&pid_show).is_some_and(|s| !s.is_empty())
                                }>
                                    <div class="mt-2 text-sm text-gray-600 dark:text-gray-300 italic flex items-center gap-2 bg-gray-50 dark:bg-gray-700/50 px-2 py-1 rounded">
                                        <span class="animate-bounce">"✎"</span>
                                        <span class="truncate">
                                            {
                                                let pid_text = pid.clone();
                                                move || typing_status.get().get(&pid_text).cloned().unwrap_or_default()
                                            }
                                        </span>
                                    </div>
                                </Show>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
