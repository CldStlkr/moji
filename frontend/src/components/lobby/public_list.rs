use leptos::prelude::*;
use shared::{get_public_lobbies, LobbySummary, LobbyId, PlayerId, JoinLobbyRequest, join_lobby};
use crate::{
    styled_view,
    persistence::SessionData,
    context::AuthContext,
};
use std::time::Duration;

styled_view!(list_container, "mt-8 space-y-4 animate-page-entry");
styled_view!(list_title, "text-xl font-bold text-gray-800 dark:text-gray-100 mb-4");
styled_view!(lobby_card, "flex justify-between items-center p-4 bg-white dark:bg-gray-800 rounded-lg shadow border border-gray-100 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-500 transition-all");
styled_view!(lobby_info, "flex flex-col");
styled_view!(lobby_name, "font-bold text-gray-800 dark:text-gray-100");
styled_view!(lobby_meta, "text-xs text-gray-500 dark:text-gray-400");
styled_view!(join_btn, "bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 text-white text-sm font-semibold py-2 px-4 rounded transition-colors disabled:opacity-50");

#[component]
pub fn PublicLobbiesList<F>(
    on_lobby_joined: F,
    set_status: WriteSignal<String>,
    set_is_loading: WriteSignal<bool>,
) -> impl IntoView
where
    F: Fn(LobbyId, PlayerId) + 'static + Copy + Send + Sync,
{
    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");
    let run_api_action = crate::hooks::use_api_action(set_is_loading, set_status);

    // Poll every 5 seconds
    let lobbies_resource = Resource::new(
        || (),
        |_| async move {
            get_public_lobbies().await.unwrap_or_default()
        }
    );

    // Automatic polling
    Effect::new(move |_| {
        let _ = set_interval_with_handle(move || {
            lobbies_resource.refetch();
        }, Duration::from_secs(5));
    });

    let join_public_lobby = move |lobby: LobbySummary| {
        let user = match auth_context.user.get() {
            Some(u) => u,
            None => {
                auth_context.set_show_auth_modal.set(true);
                return;
            }
        };

        run_api_action(Box::pin({
            let username = user.username.clone();
            let l_id = lobby.id.clone();
            async move {
                set_status.set(format!("Joining lobby {}...", l_id));
                
                let request = JoinLobbyRequest {
                    player_name: username.clone(),
                    player_id: None,
                    joining_from_public_list: true,
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

    view! {
        <Transition
            fallback=move || view! { <div class="text-center py-4 text-gray-500 italic">"Loading lobbies..."</div> }
        >
            <div class=list_container()>
                <h3 class=list_title()>"Public Lobbies"</h3>
                {move || match lobbies_resource.get() {
                    Some(lobbies) if lobbies.is_empty() => {
                        view! { <div class="text-center py-8 bg-gray-50 dark:bg-gray-700/30 rounded-lg text-gray-500 italic">"No public lobbies found. Create one!"</div> }.into_any()
                    }
                    Some(lobbies) => {
                        view! {
                            <div class="grid gap-4 sm:grid-cols-2">
                                <For
                                    each=move || lobbies.clone()
                                    key=|l| l.id.clone()
                                    children=move |l| {
                                        let lobby_to_join = l.clone();
                                        view! {
                                            <div class=lobby_card()>
                                                <div class=lobby_info()>
                                                    <span class=lobby_name()>{l.leader_name.clone()}"'s Lobby"</span>
                                                    <span class=lobby_meta()>
                                                        {format!("{:?} • {}/{} players", l.mode, l.player_count, l.max_players)}
                                                    </span>
                                                </div>
                                                <button
                                                    on:click=move |_| join_public_lobby(lobby_to_join.clone())
                                                    class=join_btn()
                                                >
                                                    "Join"
                                                </button>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    }
                    None => view! { <div class="text-center py-4 text-gray-500 italic">"..."</div> }.into_any()
                }}
            </div>
        </Transition>
    }
}
