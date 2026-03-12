// Component for managing lobby state
use crate::{
    error::{get_user_friendly_message, log_error},
    components::lobby::settings::{LobbySettingsPanel, use_lobby_settings},
    styled_view,
};
use leptos::ev;
use leptos::prelude::*;
use shared::{LobbyInfo, LobbyId, PlayerData, PlayerId, StartGameRequest, start_game};
use wasm_bindgen_futures::spawn_local;

styled_view!(lobby_container, "max-w-2xl mx-auto my-8 p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors");
styled_view!(loading_text, "text-lg text-gray-500 text-center");
styled_view!(header_container, "flex justify-between items-center mb-6 relative");
styled_view!(header_title, "text-2xl font-bold text-gray-800 dark:text-gray-100");
styled_view!(lobby_id_span, "font-mono font-bold tracking-wider text-blue-600 dark:text-blue-400");
styled_view!(copy_button, "px-2 py-1 text-xs font-medium bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors text-gray-800 dark:text-gray-200");
styled_view!(copied_toast, "absolute -top-8 left-1/2 transform -translate-x-1/2 px-2 py-1 bg-green-500 text-white text-xs rounded shadow-lg animate-fade-in pointer-events-none");

styled_view!(player_list_title, "text-xl font-semibold text-blue-600 dark:text-blue-400 border-b border-gray-200 dark:border-gray-700 pb-2");
styled_view!(player_item, is_current: bool, 
    "flex justify-between items-center p-3 rounded border-b border-gray-200 dark:border-gray-700 text-gray-800 dark:text-gray-200", 
    if is_current { "bg-blue-50 dark:bg-blue-900/30 font-semibold" } else { "" }
);

styled_view!(btn_start_game, "bg-green-500 hover:bg-green-600 dark:bg-green-600 dark:hover:bg-green-700 disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed text-white font-semibold py-3 px-6 rounded transition-colors");
styled_view!(btn_leave_lobby, "bg-transparent hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 border border-gray-400 dark:border-gray-500 font-medium py-2 px-4 rounded transition-colors");

#[component]
pub fn LobbyManagementComponent<F>(
    lobby_info: ReadSignal<Option<LobbyInfo>>,
    current_lobby_id: ReadSignal<LobbyId>,
    current_player_id: ReadSignal<PlayerId>,
    _is_loading: ReadSignal<bool>,
    set_is_loading: WriteSignal<bool>,
    _status: ReadSignal<String>,
    set_status: WriteSignal<String>,
    on_leave_lobby: F,
) -> impl IntoView
where
    F: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    let is_copied = RwSignal::new(false);
    let start_game_action = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        let player_id = current_player_id.get();

        spawn_local(async move {
            set_is_loading.set(true);
            set_status.set("Starting game...".to_string());

            let request = StartGameRequest {
                player_id: player_id.clone(),
            };

            match start_game(lobby_id.clone(), request).await {
                Ok(_) => {
                    set_status.set("Game starting...".to_string());
                }
                Err(e) => {
                    log_error("Failed to start game", e.clone());
                    set_status.set(get_user_friendly_message(e.clone()));
                }
            }
            set_is_loading.set(false);
        });
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await {
                Ok(_) => {
                    is_copied.set(true);

                    set_timeout(
                        move || {
                            is_copied.set(false);
                        },
                        std::time::Duration::from_millis(1000),
                    );
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard")
                }
            }
        });
    };

    let is_leader = move || {
        if let Some(info) = lobby_info.get() {
            info.leader_id == current_player_id.get()
        } else {
            false
        }
    };

    view! {
        <div class=lobby_container()>
            <LobbyHeader lobby_id=current_lobby_id on_copy_id=copy_lobby_id is_copied=is_copied.read_only() />
            <LobbyDetails
                lobby_info=lobby_info
                current_player_id=current_player_id
                is_leader=is_leader
                on_start_game=start_game_action
                on_leave_lobby=on_leave_lobby
            />
        </div>
    }
}

#[component]
fn LobbyHeader<F>(
    lobby_id: ReadSignal<LobbyId>,
    on_copy_id: F,
    is_copied: ReadSignal<bool>,
) -> impl IntoView
where
    F: Fn(ev::MouseEvent) + 'static + Copy,
{
    view! {
        <div class=header_container()>
            <h2 class=header_title()>
                "Lobby: "
                <span class=lobby_id_span()>
                    {move || lobby_id.get().to_string()}
                </span>
            </h2>
            <button
                on:click=on_copy_id
                class=copy_button()
                title="Copy Lobby ID"
            >
                "Copy"
            </button>
        </div>

        // Floating "Copied!" text using Show
        <Show when=move || is_copied.get()>
            <div class=copied_toast()>
                "Copied!"
            </div>
        </Show>
    }
}

#[component]
pub fn StatusMessage(status: ReadSignal<String>) -> impl IntoView {
    view! {
        <Show when=move || !status.get().is_empty()>
            <div class=move || {
                let base_classes = "my-4 p-3 rounded text-center font-medium";
                if status.get().contains("Error") {
                    format!("{} bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300", base_classes)
                } else {
                    format!("{} bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300", base_classes)
                }
            }>{move || status.get()}</div>
        </Show>
    }
}

#[component]
fn LobbyDetails<F1, F2>(
    lobby_info: ReadSignal<Option<LobbyInfo>>,
    current_player_id: ReadSignal<PlayerId>,
    is_leader: impl Fn() -> bool + 'static + Copy + Send + Sync,
    on_start_game: F1,
    on_leave_lobby: F2,
) -> impl IntoView
where
    F1: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
    F2: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    let settings = Signal::derive(move || {
        lobby_info.get()
            .map(|i| i.settings)
            .unwrap_or_default()
    });

    let lobby_id_for_settings = Signal::derive(move || {
        lobby_info.get()
            .map(|i| i.lobby_id)
            .unwrap_or_default()
    });

    let on_update = use_lobby_settings(lobby_id_for_settings, current_player_id);

    let players = Signal::derive(move || lobby_info.get().map(|i| i.players).unwrap_or_default());
    let leader_id = Signal::derive(move || lobby_info.get().map(|i| i.leader_id).unwrap_or_default());
    let player_count = Signal::derive(move || players.get().len());
    let max_players = Signal::derive(move || lobby_info.get().map(|i| i.settings.max_players).unwrap_or(4));

    view! {
        <Show
            when=move || lobby_info.get().is_some()
            fallback=|| view! { <div class=loading_text()>"Loading lobby info..."</div> }
        >
            <div class="space-y-6">
                <PlayersList
                    players=players
                    current_player_id=current_player_id
                    leader_id=leader_id
                    player_count=player_count
                    max_players=max_players
                />

                <LobbySettingsPanel
                    settings=settings
                    is_leader=is_leader
                    on_update=on_update
                />

                <LobbyActions
                    is_leader=is_leader
                    player_count=player_count
                    on_start_game=on_start_game
                    on_leave_lobby=on_leave_lobby
                />
            </div>
        </Show>
    }
}

#[component]
fn PlayersList(
    #[prop(into)] players: Signal<Vec<PlayerData>>,
    current_player_id: ReadSignal<PlayerId>,
    #[prop(into)] leader_id: Signal<PlayerId>,
    #[prop(into)] player_count: Signal<usize>,
    #[prop(into)] max_players: Signal<u32>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <h3 class=player_list_title()>
                "Players (" {player_count} "/" {max_players} ")"
            </h3>
            <ul class="space-y-2">
                <For
                    each=move || players.get()
                    key=|player| player.id.clone()
                    children=move |player| {
                        let id_for_item = player.id.clone();
                        let id_for_leader = player.id.clone();
                        let id_for_you = player.id.clone();
                        
                        view! {
                            <li class=move || player_item(id_for_item == current_player_id.get())>
                                <span class="font-medium">{player.name}</span>
                                <div class="flex items-center gap-2">
                                    <Show when=move || id_for_leader == leader_id.get()>
                                        <span class="text-lg" title="Lobby Leader">
                                            "👑"
                                        </span>
                                    </Show>
                                    <Show when=move || id_for_you == current_player_id.get()>
                                        <span class="text-sm text-blue-600 dark:text-blue-400 font-medium">
                                            "(You)"
                                        </span>
                                    </Show>
                                </div>
                            </li>
                        }
                    }
                />
            </ul>
        </div>
    }
}

#[component]
fn LobbyActions<F1, F2>(
    #[prop(into)] is_leader: Signal<bool>,
    #[prop(into)] player_count: Signal<usize>,
    on_start_game: F1,
    on_leave_lobby: F2,
) -> impl IntoView
where
    F1: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
    F2: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    let not_enough_players = Signal::derive(move || player_count.get() < 1);
    view! {
        <div class="flex flex-col gap-4 my-6">
            <Show
                when=move || is_leader.get()
                fallback=|| {
                    view! {
                        <p class="text-center text-gray-600 dark:text-gray-400 italic py-4">
                            "Waiting for leader to start the game..."
                        </p>
                    }
                }
            >
                <button
                    on:click=on_start_game
                    disabled=move || not_enough_players.get()
                    class=btn_start_game()
                >
                    "Start Game"
                </button>
                <Show when=move || not_enough_players.get()>
                    <p class="text-orange-600 dark:text-orange-400 text-center font-medium">
                        "Need at least 2 players to start"
                    </p>
                </Show>
            </Show>

            <button
                on:click=on_leave_lobby
                class=btn_leave_lobby()
            >
                "Leave Lobby"
            </button>
        </div>
    }
}



#[component]
pub fn GameInstructions() -> impl IntoView {
    view! {
        <div class="mt-8 pt-4 border-t border-gray-200">
            <h3 class="text-lg font-semibold text-gray-600 mb-3">"How to Play"</h3>
            <div class="space-y-2 text-gray-700">
                <p>"Create a new game or join an existing one with a lobby ID."</p>
                <p>"Once in a game, you'll be shown a kanji word or character
                    depending on the mode you choose."
                </p>
                <p>
                    "Type a Japanese word that contains that kanji, or type a valid hiragana reading
                    for that word, and submit it to score points!"
                </p>
            </div>
        </div>
    }
}
