// Component for joining/creating lobbies
use crate::{
    persistence::SessionData,
    context::AuthContext,
    styled_view,
};
use leptos::ev;
use leptos::prelude::*;
use shared::{JoinLobbyRequest, LobbyId, PlayerId, create_lobby, join_lobby};

use super::{GameInstructions, StatusMessage};

styled_view!(card_container, "card max-w-2xl mx-auto my-8 bg-white dark:bg-gray-800 shadow-lg p-8 rounded-lg transition-colors");
styled_view!(card_title, "text-3xl font-bold text-gray-800 dark:text-gray-100 mb-8 text-center");
styled_view!(btn_primary, "w-full text-lg bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 text-white font-semibold py-3 px-5 rounded disabled:opacity-60 disabled:cursor-not-allowed transition-all");
styled_view!(input_field, "flex-1 p-3 border-2 border-gray-300 dark:border-gray-600 dark:bg-gray-900 dark:text-white rounded-md text-base transition-colors focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none");
styled_view!(btn_secondary, "btn-secondary whitespace-nowrap bg-orange-300 hover:bg-orange-400 dark:bg-orange-600 dark:hover:bg-orange-700 text-gray-800 dark:text-gray-100 font-semibold py-3 px-4 rounded disabled:opacity-60 disabled:cursor-not-allowed transition-all");

#[component]
pub fn LobbyJoinComponent<F>(
    is_loading: ReadSignal<bool>,
    set_is_loading: WriteSignal<bool>,
    status: ReadSignal<String>,
    set_status: WriteSignal<String>,
    on_lobby_joined: F,
) -> impl IntoView
where
    F: Fn(LobbyId, PlayerId) + 'static + Copy + Send + Sync,
{
    let input_lobby_id = RwSignal::new(String::new());
    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");

    let run_api_action = crate::hooks::use_api_action(set_is_loading, set_status);

    let create_lobby_action = move |_: ev::MouseEvent| {
        let user = match auth_context.user.get() {
            Some(u) => u,
            None => {
                auth_context.set_show_auth_modal.set(true);
                return;
            }
        };

        run_api_action(Box::pin({
            let username = user.username.clone();
            let request = JoinLobbyRequest {
                player_name: username.clone(),
                player_id: None,
            };

            async move {
                set_status.set("Creating lobby...".to_string());
                let response = create_lobby(request).await?;
                
                let lobby_id = LobbyId::from(
                    response.get("lobby_id").and_then(|id| id.as_str()).unwrap_or("").to_string()
                );
                let player_id = PlayerId::from(
                    response.get("player_id").and_then(|id| id.as_str()).unwrap_or("")
                );

                if lobby_id.is_empty() || player_id.is_empty() {
                    return Err(crate::error::ClientError::Data("Invalid response from server".to_string()));
                }

                let session = SessionData {
                    lobby_id: lobby_id.clone(),
                    player_id: player_id.clone(),
                    player_name: username,
                    is_in_game: false,
                };
                crate::persistence::save_session(&session);
                on_lobby_joined(lobby_id, player_id);
                Ok(())
            }
        }));
    };

    let join_lobby_action = move |_: ev::MouseEvent| {
        let lobby_id = LobbyId::from(input_lobby_id.get());

        if lobby_id.trim().is_empty() {
            set_status.set("Please enter a lobby ID".to_string());
            return;
        }

        let user = match auth_context.user.get() {
            Some(u) => u,
            None => {
                 auth_context.set_show_auth_modal.set(true);
                 return;
            }
        };

        run_api_action(Box::pin({
            let username = user.username.clone();
            let l_id = lobby_id.clone();
            
            async move {
                set_status.set(format!("Joining lobby {}...", l_id));
                
                let session = crate::persistence::load_session();
                let player_id_opt = if let Some(s) = session {
                    if s.lobby_id.to_string() == l_id.to_string() {
                        Some(s.player_id)
                    } else { None }
                } else { None };

                let request = JoinLobbyRequest {
                    player_name: username.clone(),
                    player_id: player_id_opt,
                };

                let response = join_lobby(l_id.clone(), request).await?;
                let player_id = PlayerId::from(
                    response.get("player_id").and_then(|id| id.as_str()).unwrap_or("")
                );

                if player_id.0.is_empty() {
                    return Err(crate::error::ClientError::Data("Invalid response from server".to_string()));
                }

                let session = SessionData {
                    lobby_id: l_id.clone(),
                    player_id: player_id.clone(),
                    player_name: username,
                    is_in_game: false,
                };
                crate::persistence::save_session(&session);
                on_lobby_joined(l_id, player_id);
                Ok(())
            }
        }));
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            join_lobby_action(ev::MouseEvent::new("click").unwrap());
        }
    };

    view! {
        <div class=card_container()>
            <h2 class=card_title()>
                "Join or Create a Game"
            </h2>
            <div class="space-y-6">
                // Removed player name input

                <button
                    on:click=create_lobby_action
                    disabled=move || is_loading.get()
                    class=btn_primary()
                >
                    "Create New Game"
                </button>

                <div class="flex gap-3 flex-col sm:flex-row">
                    <input
                        type="text"
                        value=move || input_lobby_id.get()
                        on:input=move |ev| input_lobby_id.set(event_target_value(&ev))
                        on:keydown=handle_key_press
                        placeholder="Enter Lobby ID"
                        disabled=move || is_loading.get()
                        class=input_field()
                    />
                    <button
                        on:click=join_lobby_action
                        disabled=move || {
                            is_loading.get() || input_lobby_id.get().trim().is_empty()
                        }
                        class=btn_secondary()
                    >
                        "Join Game"
                    </button>
                </div>
            </div>

            <StatusMessage status=status />
            <GameInstructions />
        </div>
    }
}
