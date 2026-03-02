use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;

use crate::{
    components::{
        game::GameComponent,
        lobby::LobbyManagementComponent,
        shared_socket::{use_shared_socket, UseSharedSocketConfig},
    },
    context::AuthContext,
    error::{get_user_friendly_message, log_error},
    persistence::{clear_session, load_session, save_session, use_session_persistence, SessionData},
};
use shared::{get_lobby_info, get_player_info, join_lobby, leave_lobby, JoinLobbyRequest, LobbyId, LobbyInfo, PlayerId};

#[component]
pub fn LobbyPage() -> impl IntoView {
    let params = use_params_map();
    let url_lobby_id = move || params.get().get("id").unwrap_or_default();

    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");

    // Core state
    let lobby_id = RwSignal::new(LobbyId::default());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());

    // Game/Lobby State
    let lobby_info = RwSignal::new(None::<LobbyInfo>);
    let prompt = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    let typing_status = RwSignal::new(HashMap::<PlayerId, String>::new());

    // UI State
    let is_restoring = RwSignal::new(true);
    let (join_status, set_join_status) = signal(String::new());
    let (is_joining, set_is_joining) = signal(false);

    let navigate = use_navigate();
    let navigate_path = RwSignal::new(None::<String>);
    let navigate_replace_path = RwSignal::new(None::<String>);

    let navigate_effect = navigate.clone();
    Effect::new(move |_| {
         if let Some(path) = navigate_path.get() {
             navigate_effect(&path, Default::default());
        }
    });

    let navigate_replace_effect = navigate.clone();
    Effect::new(move |_| {
         if let Some(path) = navigate_replace_path.get() {
             navigate_replace_effect(&path, leptos_router::NavigateOptions {
                 replace: true,
                 ..Default::default()
             });
        }
    });

    let send_message = use_shared_socket(UseSharedSocketConfig {
        lobby_id: lobby_id.read_only(),
        player_id: player_id.read_only(),
        set_lobby_info: lobby_info.write_only(),
        set_prompt: prompt.write_only(),
        set_result: result.write_only(),
        set_typing_status: typing_status.write_only(),
    });

    let is_in_game = Signal::derive(move || {
        lobby_info.get().map(|info| 
            info.status == shared::GameStatus::Playing ||
            info.status == shared::GameStatus::Finished
        ).unwrap_or(false)
    });

    use_session_persistence(
        lobby_id.read_only(),
        player_id.read_only(),
        player_name.read_only(),
        is_in_game,
    );

    // Initial load and session check
    Effect::new(move |_| {
        spawn_local(async move {
            let id = url_lobby_id();
            if id.is_empty() {
                navigate_replace_path.set(Some("/".to_string()));
                return;
            }

            if let Some(session_data) = load_session() {
                if session_data.lobby_id.to_string() == id {
                    // Valid session for this lobby. Load user info.
                    match get_player_info(session_data.lobby_id.clone(), session_data.player_id.clone()).await {
                        Ok(player_info) => {
                            lobby_id.set(session_data.lobby_id.clone());
                            player_id.set(session_data.player_id.clone());
                            player_name.set(player_info.name);

                            if let Ok(info) = get_lobby_info(session_data.lobby_id.clone()).await {
                                 lobby_info.set(Some(info));
                            }

                            is_restoring.set(false);
                            return;
                        }
                        Err(_) => {
                            clear_session();
                            // Fall through to show Join UI
                        }
                    }
                }
            }

            // No session or wrong session, show join UI
            is_restoring.set(false);
        });
    });

    // Auto-join effect when user logs in via Auth modal while sitting on the Join UI
    Effect::new(move |_| {
        let id = url_lobby_id();
        if id.is_empty() || is_restoring.get() || !lobby_id.get().is_empty() { 
            return; // Only run if we don't have a valid sitting session and haven't loaded yet
        }

        if let Some(user) = auth_context.user.get() {
            spawn_local(async move {
                set_is_joining.set(true);
                set_join_status.set(format!("Joining lobby {} as {}...", id, user.username));

                let request = JoinLobbyRequest { player_name: user.username.clone() };
                let join_lobby_id = LobbyId::from(id.clone());

                match join_lobby(join_lobby_id.clone(), request).await {
                    Ok(response) => {
                         let new_player_id = PlayerId::from(
                            response.get("player_id").and_then(|pid| pid.as_str()).unwrap_or("")
                        );

                        if new_player_id.0.is_empty() {
                             set_join_status.set("Invalid response from server".to_string());
                             set_is_joining.set(false);
                        } else {
                            let session = SessionData {
                                lobby_id: join_lobby_id.clone(),
                                player_id: new_player_id.clone(),
                                player_name: user.username.clone(),
                                is_in_game: false,
                            };
                            save_session(&session);

                            // Trigger complete state hydrate to enter lobby mode
                            lobby_id.set(join_lobby_id.clone());
                            player_id.set(new_player_id.clone());
                            player_name.set(user.username.clone());

                            if let Ok(info) = get_lobby_info(join_lobby_id.clone()).await {
                                lobby_info.set(Some(info));
                            }

                            set_is_joining.set(false);
                        }
                    }
                    Err(e) => {
                        log_error("Failed to join lobby via link", e.clone());
                        set_join_status.set(get_user_friendly_message(e.clone()));
                        set_is_joining.set(false);
                    }
                }
            });
        } 
    });

    let handle_guest_join = move |_| {
        spawn_local(async move {
            set_is_joining.set(true);
            let random_suffix: String = (0..4).map(|_| {
                let idx = (js_sys::Math::random() * 10.0) as usize;
                idx.to_string()
            }).collect();
            let guest_name = format!("Guest{}", random_suffix);

            match crate::context::create_guest_account(guest_name.clone()).await {
                Ok(final_username) => {
                    auth_context.set_user.set(Some(crate::context::User {
                        username: final_username,
                        is_guest: true,
                    }));
                }
                Err(e) => {
                    set_join_status.set(format!("Failed to create guest: {}", e));
                    set_is_joining.set(false);
                }
            }
        });
    };

    let handle_leave_and_cleanup = move || {
        let current_lobby_id = lobby_id.get_untracked();
        let current_player_id = player_id.get_untracked();

        spawn_local(async move {
            let _ = leave_lobby(current_lobby_id, current_player_id).await;

            lobby_id.set(LobbyId::default());
            player_id.set(PlayerId::default());
            player_name.set(String::new());
            lobby_info.set(None);

            clear_session();
            navigate_replace_path.set(Some("/".to_string()));
        });
    };

    let (mgmt_is_loading, mgmt_set_is_loading) = signal(false);
    let (mgmt_status, mgmt_set_status) = signal(String::new());

    view! {
        <Show
            when=move || is_restoring.get()
            fallback=move || {
                view! {
                    <Show
                        when=move || !lobby_id.get().is_empty()
                        // If we DON'T have a lobby id resolved in state, we show the join UI
                        fallback=move || {
                            view! {
                                <div class="max-w-md mx-auto mt-20 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-xl text-center">
                                    <h2 class="text-2xl font-bold mb-4 text-gray-800 dark:text-gray-100">
                                        "Join Game"
                                    </h2>
                                    <p class="mb-6 text-gray-600 dark:text-gray-300">
                                        "You've been invited to join lobby " 
                                        <span class="font-mono font-bold text-blue-500">{url_lobby_id()}</span>
                                    </p>

                                    <Show when=move || !join_status.get().is_empty()>
                                        <div class="mb-4 p-3 bg-red-100 text-red-700 rounded text-sm">
                                            {move || join_status.get()}
                                        </div>
                                    </Show>

                                    <Show
                                        when=move || is_joining.get()
                                        fallback=move || {
                                            view! {
                                                <div class="space-y-4">
                                                    <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                                        "Please log in or continue as a guest to join."
                                                    </p>

                                                    <button
                                                        on:click=move |_| auth_context.set_show_auth_modal.set(true)
                                                        class="w-full bg-blue-500 hover:bg-blue-600 text-white font-semibold py-2 px-4 rounded transition-colors"
                                                    >
                                                        "Log In"
                                                    </button>

                                                    <div class="relative flex py-2 items-center">
                                                        <div class="flex-grow border-t border-gray-300 dark:border-gray-600"></div>
                                                        <span class="flex-shrink-0 mx-4 text-gray-400 text-sm">"OR"</span>
                                                        <div class="flex-grow border-t border-gray-300 dark:border-gray-600"></div>
                                                    </div>

                                                    <button
                                                        on:click=handle_guest_join
                                                        class="w-full bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100 font-semibold py-2 px-4 rounded transition-colors"
                                                    >
                                                        "Play as Guest"
                                                    </button>
                                                </div>
                                            }
                                        }
                                    >
                                        <div class="flex flex-col items-center justify-center space-y-3 py-8">
                                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                                            <p class="text-gray-500 text-sm">{move || join_status.get()}</p>
                                        </div>
                                    </Show>
                                </div>
                            }
                        }
                    >
                        // If we DO have a lobby id securely locked in state, we show the lobby UI (settings or game map)
                        <Show
                            when=move || !is_in_game.get()
                            fallback=move || {
                                view! {
                                    <GameComponent
                                        lobby_id=lobby_id.read_only()
                                        player_id=player_id.read_only()
                                        on_exit_game=handle_leave_and_cleanup
                                        send_message=send_message
                                        prompt=prompt.read_only()
                                        result=result.read_only()
                                        typing_status=typing_status
                                        lobby_info=lobby_info.read_only()
                                    />
                                }
                            }
                        >
                            <LobbyManagementComponent 
                                lobby_info=lobby_info.read_only()
                                current_lobby_id=lobby_id.read_only()
                                current_player_id=player_id.read_only()
                                _is_loading=mgmt_is_loading
                                set_is_loading=mgmt_set_is_loading
                                _status=mgmt_status
                                set_status=mgmt_set_status
                                on_leave_lobby=move |_| handle_leave_and_cleanup()
                            />
                        </Show>
                    </Show>
                }
            }
        >
            <div class="text-center p-8 mt-20">
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-4"></div>
                <div class="text-lg text-gray-600 dark:text-gray-300">"Loading..."</div>
            </div>
        </Show>
    }
}
