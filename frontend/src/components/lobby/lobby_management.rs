// Component for managing lobby state
use crate::{
    components::lobby::{settings::{LobbySettingsPanel, use_lobby_settings}, ChatComponent},
    styled_view,
    context::GameContext,
    components::toast::{use_toast, ToastType},
};
use leptos::ev;
use leptos::prelude::*;
use shared::{StartGameRequest, GameMode, start_game};
use wasm_bindgen_futures::spawn_local;

styled_view!(lobby_container, "max-w-2xl mx-auto my-8 p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors");
styled_view!(loading_text, "text-lg text-gray-500 text-center");
styled_view!(header_container, "flex justify-between items-center mb-6 relative");
styled_view!(header_title, "text-2xl font-bold text-gray-800 dark:text-gray-100");
styled_view!(lobby_id_span, "font-mono font-bold tracking-wider text-blue-600 dark:text-blue-400");
styled_view!(copy_button, "px-2 py-1 text-xs font-medium bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors text-gray-800 dark:text-gray-200");

styled_view!(player_list_title, "text-xl font-semibold text-blue-600 dark:text-blue-400 border-b border-gray-200 dark:border-gray-700 pb-2");
styled_view!(player_item, is_current: bool, 
    "flex justify-between items-center p-3 rounded border-b border-gray-200 dark:border-gray-700 text-gray-800 dark:text-gray-200", 
    if is_current { "bg-blue-50 dark:bg-blue-900/30 font-semibold" } else { "" }
);

styled_view!(btn_start_game, "bg-green-500 hover:bg-green-600 dark:bg-green-600 dark:hover:bg-green-700 disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed text-white font-semibold py-3 px-6 rounded transition-colors");
styled_view!(btn_leave_lobby, "bg-transparent hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 border border-gray-400 dark:border-gray-500 font-medium py-2 px-4 rounded transition-colors");

#[component]
pub fn LobbyManagementComponent(
    #[prop(into)] on_leave_lobby: Callback<ev::MouseEvent>, // Use Callback for cleaner prop
    set_is_loading: WriteSignal<bool>,
    set_status: WriteSignal<String>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    
    let start_game_action = {
        let lobby_id = game_context.lobby_id;
        let player_id = game_context.player_id;
        let run_api_action = crate::hooks::use_api_action(set_is_loading, set_status);

        move |_: ev::MouseEvent| {
            let l_id = lobby_id.get();
            let p_id = player_id.get();

            run_api_action(Box::pin({
                async move {
                    set_status.set("Starting game...".to_string());
                    let request = StartGameRequest { player_id: p_id };
                    let _ = start_game(l_id, request).await?;
                    Ok(())
                }
            }));
        }
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let toast = use_toast();
        let lobby_id = game_context.lobby_id.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await {
                Ok(_) => {
                    toast.push.run(("Lobby ID copied to clipboard!".to_string(), ToastType::Success));
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard");
                }
            }
        });
    };

    view! {
        <div class=lobby_container()>
            <LobbyHeader on_copy_id=copy_lobby_id />
            <LobbyDetails
                on_start_game=Callback::new(start_game_action)
                on_leave_lobby=on_leave_lobby
                set_is_loading=set_is_loading
                set_status=set_status
            />
        </div>
    }
}

#[component]
fn LobbyHeader(
    #[prop(into)] on_copy_id: Callback<ev::MouseEvent>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let lobby_id = game_context.lobby_id;

    view! {
        <div class=header_container()>
            <h2 class=header_title()>
                "Lobby: "
                <span class=lobby_id_span()>
                    {move || lobby_id.get().to_string()}
                </span>
            </h2>
            <button
                on:click=move |ev| on_copy_id.run(ev)
                class=copy_button()
                title="Copy Lobby ID"
            >
                "Copy"
            </button>
        </div>
    }
}

#[component]
pub fn StatusMessage(status: ReadSignal<String>) -> impl IntoView {
    view! {
        <Show when=move || !status.get().is_empty()>
            <div class=move || {
                let base_classes = "my-4 p-3 rounded text-center font-medium";
                if status.get().contains("Error") || status.get().contains("Invalid") || status.get().contains("Failed") {
                    format!("{} bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300", base_classes)
                } else {
                    format!("{} bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300", base_classes)
                }
            }>{move || status.get()}</div>
        </Show>
    }
}

#[component]
fn LobbyDetails(
    #[prop(into)] on_start_game: Callback<ev::MouseEvent>,
    #[prop(into)] on_leave_lobby: Callback<ev::MouseEvent>,
    set_is_loading: WriteSignal<bool>,
    set_status: WriteSignal<String>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let lobby_info = game_context.lobby_info;

    let settings = Signal::derive(move || {
        lobby_info.get()
            .map(|i| i.settings)
            .unwrap_or_default()
    });

    let on_update = use_lobby_settings(set_is_loading, set_status);

    view! {
        <Show
            when=move || lobby_info.get().is_some()
            fallback=|| view! { <div class=loading_text()>"Loading lobby info..."</div> }
        >
            <div class="space-y-6">
                <PlayersList set_is_loading=set_is_loading set_status=set_status />
                <ChatComponent />
                <LobbySettingsPanel settings=settings on_update=on_update />
                <LobbyActions on_start_game=on_start_game on_leave_lobby=on_leave_lobby />
            </div>
        </Show>
    }
}

#[component]
fn PlayersList(set_is_loading: WriteSignal<bool>, set_status: WriteSignal<String>) -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let lobby_info = game_context.lobby_info;
    let current_player_id = game_context.player_id;
    let is_leader = game_context.is_leader;
    let run_api_action = crate::hooks::use_api_action(set_is_loading, set_status);

    let players = Signal::derive(move || lobby_info.get().map(|i| i.players).unwrap_or_default());
    let leader_id = Signal::derive(move || lobby_info.get().map(|i| i.leader_id).unwrap_or_default());
    let player_count = Signal::derive(move || players.get().len());
    let max_players = Signal::derive(move || lobby_info.get().map(|i| i.settings.max_players).unwrap_or(4));

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
                        let id_for_actions = player.id.clone();

                        let on_kick = {
                            let r_id = current_player_id.get();
                            let t_id = id_for_actions.clone();
                            let run_api = run_api_action;
                            move |_| {
                                let t_id = t_id.clone();
                                let l_id = lobby_info.get().map(|i| i.lobby_id).unwrap_or_default();
                                let r_id = r_id.clone();
                                run_api(Box::pin(async move {
                                    shared::kick_player(l_id, r_id, t_id).await?;
                                    Ok(())
                                }));
                            }
                        };

                        let on_promote = {
                            let r_id = current_player_id.get();
                            let t_id = id_for_actions.clone();
                            let run_api = run_api_action;
                            move |_| {
                                let t_id = t_id.clone();
                                let l_id = lobby_info.get().map(|i| i.lobby_id).unwrap_or_default();
                                let r_id = r_id.clone();
                                run_api(Box::pin(async move {
                                    shared::promote_leader(l_id, r_id, t_id).await?;
                                    Ok(())
                                }));
                            }
                        };

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
                                    <Show when=move || is_leader.get() && id_for_actions != current_player_id.get()>
                                        <button
                                            on:click=on_kick.clone()
                                            class="px-2 py-1 text-xs font-semibold rounded bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-800 transition-colors"
                                            title="Kick Player"
                                        >
                                            "Kick"
                                        </button>
                                        <button
                                            on:click=on_promote.clone()
                                            class="px-2 py-1 text-xs font-semibold rounded bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 hover:bg-blue-200 dark:hover:bg-blue-800 transition-colors"
                                            title="Make Leader"
                                        >
                                            "Make Leader"
                                        </button>
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
fn LobbyActions(
    #[prop(into)] on_start_game: Callback<ev::MouseEvent>,
    #[prop(into)] on_leave_lobby: Callback<ev::MouseEvent>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let is_leader = game_context.is_leader;
    let lobby_info = game_context.lobby_info;

    let player_count = Signal::derive(move || lobby_info.get().map(|i| i.players.len()).unwrap_or(0));
    let game_mode = Signal::derive(move || lobby_info.get().map(|i| i.settings.mode).unwrap_or(GameMode::Zen));

    let not_enough_players = Signal::derive(move || match game_mode.get() {
        GameMode::Duel | GameMode::Deathmatch => player_count.get() < 2 ,
        _ => false,
    });

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
                    on:click=move |ev| on_start_game.run(ev)
                    disabled=move || not_enough_players.get()
                    class=btn_start_game()
                >
                    "Start Game"
                </button>
                <Show when=move || not_enough_players.get()>
                    <p class="text-orange-600 dark:text-orange-400 text-center font-medium">
                        {move || match game_mode.get() {
                            GameMode::Duel | GameMode::Deathmatch => "Need at least 2 players to start",
                            _ => "",
                        }}
                    </p>
                </Show>
            </Show>

            <button
                on:click=move |ev| on_leave_lobby.run(ev)
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
