use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use crate::{
    api::{join_lobby, create_guest_account}, 
    context::AuthContext, 
    error::{get_user_friendly_message, log_error},
    persistence::SessionData,
    components::home::Home,
};
use shared::{JoinLobbyRequest, PlayerId};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn JoinHandler() -> impl IntoView {
    let params = use_params_map();
    let lobby_id = move || params.get().get("id").unwrap_or_default();


    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");

    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (view_game, set_view_game) = signal(false);

    // Reactively check for session match once lobby_id param is available
    Effect::new(move |_| {
        let id = lobby_id();
        if !id.is_empty() {
            if let Some(session) = crate::persistence::load_session() {
                if session.lobby_id == id {
                    set_view_game.set(true);
                }
            }
        }
    });

    // Auto-join effect when user is authenticated
    Effect::new(move |_| {
        let id = lobby_id();
        if id.is_empty() {
             return;
        }

        if let Some(user) = auth_context.user.get() {
            // User is authenticated, proceed to join
            spawn_local(async move {
                // Only try joining if not already viewing game
                if view_game.get_untracked() {
                    return;
                }

                set_is_loading.set(true);
                set_status.set(format!("Joining lobby {} as {}...", id, user.username));

                let request = JoinLobbyRequest {
                    player_name: user.username.clone(),
                };

                match join_lobby(&id, request).await {
                    Ok(response) => {
                         let player_id = PlayerId::from(
                            response
                                .get("player_id")
                                .and_then(|pid| pid.as_str())
                                .unwrap_or("")
                        );

                        if player_id.0.is_empty() {
                             set_status.set("Invalid response from server".to_string());
                             set_is_loading.set(false);
                        } else {
                             // Save session
                            let session = SessionData {
                                lobby_id: id.clone(),
                                player_id: player_id.clone(),
                                player_name: user.username.clone(),
                                is_in_game: false,
                            };
                            crate::persistence::save_session(&session);

                            // Render Game instead of navigating
                            set_view_game.set(true);
                        }
                    }
                    Err(e) => {
                        log_error("Failed to join lobby via link", &e);
                        set_status.set(get_user_friendly_message(&e));
                        set_is_loading.set(false);
                    }
                }
            });
        } 
        // If not authenticated, we wait for user action (login or guest)
    });

    let handle_guest_join = move |_| {
        spawn_local(async move {
            set_is_loading.set(true);
            let random_suffix: String = (0..4).map(|_| {
                let idx = (js_sys::Math::random() * 10.0) as usize;
                idx.to_string()
            }).collect();
            let guest_name = format!("Guest{}", random_suffix);

            match create_guest_account(&guest_name).await {
                Ok(user) => {
                    // Update Auth Context - this will trigger the Effect above to join!
                    auth_context.set_user.set(Some(crate::context::User {
                        username: user["username"].as_str().unwrap_or(&guest_name).to_string(),
                        is_guest: true,
                    }));
                }
                Err(e) => {
                    set_status.set(format!("Failed to create guest: {}", e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    view! {
        <Show
            when=move || view_game.get()
            fallback=move || {
                view! {
                    <div class="max-w-md mx-auto mt-20 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-xl text-center">
                        <h2 class="text-2xl font-bold mb-4 text-gray-800 dark:text-gray-100">
                            "Join Game"
                        </h2>
                        <p class="mb-6 text-gray-600 dark:text-gray-300">
                            "You've been invited to join lobby " 
                            <span class="font-mono font-bold text-blue-500">{lobby_id}</span>
                        </p>

                        <Show when=move || !status.get().is_empty()>
                            <div class="mb-4 p-3 bg-red-100 text-red-700 rounded text-sm">
                                {move || status.get()}
                            </div>
                        </Show>

                        <Show
                            when=move || is_loading.get()
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
                                <p class="text-gray-500 text-sm">{move || status.get()}</p>
                            </div>
                        </Show>
                    </div>
                }
            }
        >
            <Home />
        </Show>
    }
}
