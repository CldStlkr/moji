use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use wasm_bindgen_futures::spawn_local;

use crate::{
    context::GameContext,
    components::{
        game::GameComponent,
        lobby::LobbyManagementComponent,
        shared_socket::{use_shared_socket, UseSharedSocketConfig},
    },
    context::AuthContext,
    persistence::{clear_session, load_session, save_session, use_session_persistence, SessionData},
};
use shared::{get_lobby_info, get_player_info, join_lobby, JoinLobbyRequest, LobbyId, LobbyInfo, PlayerId};
#[component]
pub fn LobbyPage() -> impl IntoView {
    let params = use_params_map();
    let url_lobby_id = move || params.get().get("id").unwrap_or_default();

    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");

    // Game/Lobby State (Contextual)
    let lobby_id = RwSignal::new(LobbyId::default());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let lobby_info = RwSignal::new(None::<LobbyInfo>);

    let navigate = use_navigate();

    let is_leader = Memo::new(move |_| {
        if let (Some(info), p_id) = (lobby_info.get(), player_id.get()) {
            info.leader_id == p_id && !p_id.0.is_empty()
        } else {
            false
        }
    });

    let prompt = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    let typing_status = RwSignal::new(std::collections::HashMap::<shared::PlayerId, String>::new());
    let chat_messages = RwSignal::new(Vec::<shared::ChatMessage>::new());

    let navigate_kick = navigate.clone();
    let send_message = use_shared_socket(UseSharedSocketConfig {
        lobby_id: lobby_id.read_only(),
        player_id: player_id.read_only(),
        lobby_info,
        set_prompt: prompt.write_only(),
        set_result: result.write_only(),
        set_typing_status: typing_status.write_only(),
        chat_messages,
        on_kicked: Some(Callback::new(move |_| {
            navigate_kick("/", Default::default());
        })),
    });

    provide_context(GameContext {
        lobby_id: lobby_id.read_only(),
        set_lobby_id: lobby_id.write_only(),
        player_id: player_id.read_only(),
        set_player_id: player_id.write_only(),
        player_name: player_name.read_only(),
        set_player_name: player_name.write_only(),
        lobby_info: lobby_info.read_only(),
        set_lobby_info: lobby_info.write_only(),
        is_leader,
        prompt: prompt.read_only(),
        set_prompt: prompt.write_only(),
        result: result.read_only(),
        set_result: result.write_only(),
        typing_status: typing_status.read_only(),
        set_typing_status: typing_status.write_only(),
        chat_messages,
        send_message: Callback::new(send_message),
    });

    // UI State
    let is_leaving = RwSignal::new(false);
    let (join_status, set_join_status) = signal(String::new());
    let (is_joining, set_is_joining) = signal(false);

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

    // Data Resource for initial Page Hydration
    let lobby_data_resource = Resource::new(
        url_lobby_id,
        move |id| async move {
            if id.is_empty() {
                return None;
            }

            let l_id = LobbyId::from(id.clone());
            let session = load_session().filter(|s| s.lobby_id == l_id);

            let mut info = None;
            let mut p_info = None;

            if let Ok(l_info) = get_lobby_info(l_id.clone()).await {
                info = Some(l_info);
            }

            if let Some(s) = &session {
                if let Ok(pi) = get_player_info(s.lobby_id.clone(), s.player_id.clone()).await {
                    p_info = Some(pi);
                }
            }

            Some((info, p_info, session))
        }
    );

    // Sync Resource to Signals
    Effect::new(move |_| {
        if let Some(Some((info, p_info, session))) = lobby_data_resource.get() {
            if let Some(i) = info {
                lobby_info.set(Some(i));
            }
            if let Some(pi) = p_info {
                player_name.set(pi.name);
                if let Some(s) = session {
                    lobby_id.set(s.lobby_id);
                    player_id.set(s.player_id);
                }
            } else {
                // No session or invalid session
                clear_session();
            }
        }
    });

    let handle_leave_and_cleanup = move || {
        // Prevent auto-join Effect from re-joining before navigation completes
        is_leaving.set(true);

        // CLEAR SESSION IN BACKEND
        let l_id = lobby_id.get_untracked();
        let p_id = player_id.get_untracked();
        if !l_id.0.is_empty() && !p_id.0.is_empty() {
             spawn_local(async move {
                 let _ = shared::leave_lobby(l_id, p_id).await;
             });
        }

        // Clear session immediately so no restore can happen
        clear_session();

        // Navigate away FIRST - this will unmount the component and dispose
        // all Effects (including auto-join) before lobby_id changes can trigger them
        navigate_replace_path.set(Some("/".to_string()));

        // Clear signals after setting navigate (they fire in same microtask batch)
        lobby_id.set(LobbyId::default());
        player_id.set(PlayerId::default());
        player_name.set(String::new());
        lobby_info.set(None);
    };

    let run_api_action = crate::hooks::use_api_action(set_is_joining, set_join_status);

    // Auto-join effect when user logs in via Auth modal while sitting on the Join UI
    Effect::new(move |_| {
        let id = url_lobby_id();
        if id.is_empty() || is_leaving.get() || !lobby_id.get().is_empty() { 
            return; 
        }

        // Only run if the resource has finished loading
        if lobby_data_resource.get().is_none() {
            return;
        }

        if let Some(user) = auth_context.user.get() {
            run_api_action(Box::pin({
                let u_name = user.username.clone();
                let l_id_str = id.clone();
                async move {
                    set_join_status.set(format!("Joining lobby {} as {}...", l_id_str, u_name));

                    let request = JoinLobbyRequest {
                        player_name: u_name.clone(),
                        player_id: load_session().map(|s| s.player_id),
                        joining_from_public_list: false,
                    };
                    let join_lobby_id = LobbyId::from(l_id_str);

                    let response = join_lobby(join_lobby_id.clone(), request).await?;
                    let new_player_id = PlayerId::from(
                        response.get("player_id").and_then(|pid| pid.as_str()).unwrap_or("")
                    );

                    if new_player_id.0.is_empty() {
                         return Err(crate::error::ClientError::Data("Invalid response from server".to_string()));
                    }

                    let session = SessionData {
                        lobby_id: join_lobby_id.clone(),
                        player_id: new_player_id.clone(),
                        player_name: u_name.clone(),
                        is_in_game: false,
                    };
                    save_session(&session);

                    // Trigger complete state hydrate to enter lobby mode
                    lobby_id.set(join_lobby_id.clone());
                    player_id.set(new_player_id.clone());
                    player_name.set(u_name);

                    if let Ok(info) = get_lobby_info(join_lobby_id).await {
                        lobby_info.set(Some(info));
                    }
                    Ok(())
                }
            }));
        }
    });

    // Auto-cleanup on logout
    Effect::new(move |_| {
        if auth_context.user.get().is_none() && !lobby_id.get().is_empty() && !is_leaving.get() {
            leptos::logging::log!("User logged out while in lobby, triggering cleanup...");
            handle_leave_and_cleanup();
        }
    });

    let handle_guest_join = move |_| {
        run_api_action(Box::pin({
            async move {
                let random_suffix: String = (0..4).map(|_| {
                    let idx = (js_sys::Math::random() * 10.0) as usize;
                    idx.to_string()
                }).collect();
                let guest_name = format!("Guest{}", random_suffix);

                let (final_username, token) = crate::context::create_guest_account(guest_name).await
                    .map_err(|e| crate::error::ClientError::Auth(e.to_string()))?;

                auth_context.set_user.set(Some(crate::context::User {
                    username: final_username.clone(),
                    is_guest: true,
                }));
                crate::persistence::save_auth(&crate::persistence::AuthData {
                    username: final_username,
                    is_guest: true,
                    token,
                });

                Ok(())
            }
        }));
    };



    let (_mgmt_is_loading, mgmt_set_is_loading) = signal(false);
    let (_mgmt_status, mgmt_set_status) = signal(String::new());

    view! {
        <Transition
            fallback=move || view! {
                <div class="text-center p-8 mt-20">
                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-4"></div>
                    <div class="text-lg text-gray-600 dark:text-gray-300">"Loading..."</div>
                </div>
            }
        >
            {move || lobby_data_resource.get().map(|_| {
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
                                    <div class="space-y-6 animate-page-entry">
                                        <GameComponent 
                                            on_exit_game=Callback::new(move |_| handle_leave_and_cleanup())
                                        />
                                    </div>
                                }
                            }
                        >
                            <div class="animate-page-entry">
                                <LobbyManagementComponent 
                                    set_is_loading=mgmt_set_is_loading
                                    set_status=mgmt_set_status
                                    on_leave_lobby=move |_| handle_leave_and_cleanup()
                                />
                            </div>
                        </Show>
                    </Show>
                }
            })}
        </Transition>
    }
}
