use leptos::prelude::*;
use shared::PlayerId;
use wasm_bindgen_futures::spawn_local;

use crate::{
    api,
    components::{game::GameComponent, lobby::LobbyComponent, shared_socket::{use_shared_socket, UseSharedSocketConfig}},
    persistence::{clear_session, load_session, use_session_persistence},
};
use shared::LobbyInfo;
use std::collections::HashMap;

#[component]
pub fn Home() -> impl IntoView {
    let lobby_id = RwSignal::new(String::new());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let is_restoring = RwSignal::new(true);

    // Global Lobby State
    let lobby_info = RwSignal::new(None::<LobbyInfo>);

    // Global Game State
    let prompt = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    let typing_status = RwSignal::new(HashMap::<PlayerId, String>::new());

    // Setup Shared Socket Connection
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

    // Check for existing session on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Some(session_data) = load_session() {
                match api::get_player_info(&session_data.lobby_id, &session_data.player_id).await {
                    Ok(player_info) => {
                        lobby_id.set(session_data.lobby_id.clone());
                        player_id.set(session_data.player_id);
                        player_name.set(player_info.name);

                        // Because is_in_game is now derived from lobby_info, we need to fetch lobby_info 
                        // here to properly restore the state.
                        if let Ok(info) = api::get_lobby_info(&session_data.lobby_id).await {
                             lobby_info.set(Some(info));
                        }
                    }
                    Err(_) => {
                        clear_session();
                    }
                }
            }
            is_restoring.set(false);
        });
    });

    let navigate = leptos_router::hooks::use_navigate();
    let navigate_path = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if let Some(path) = navigate_path.get() {
            navigate(&path, Default::default());
        }
    });

    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        lobby_id.set(new_lobby_id.clone());
        player_id.set(new_player_id);
        // We no longer set is_in_game=true here manually; it happens when server sends GameState
        navigate_path.set(Some(format!("/join/{}", new_lobby_id)));
    };

    let handle_leave_and_cleanup = move || {
        let current_lobby_id = lobby_id.get_untracked();
        let current_player_id = player_id.get_untracked();

        spawn_local(async move {
            let _ = api::leave_lobby(&current_lobby_id, &current_player_id).await;

            lobby_id.set(String::new());
            player_id.set(PlayerId::default());
            player_name.set(String::new());
            lobby_info.set(None);

            clear_session();
            navigate_path.set(Some("/".to_string()));
        });
    };

    view! {
        <Show
            when=move || is_restoring.get()
            fallback=move || {
                view! {
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
                    <LobbyComponent 
                        on_lobby_joined=handle_lobby_joined 
                        initial_lobby_id={
                            let id = lobby_id.get();
                            if id.is_empty() { None } else { Some(id) }
                        }
                        initial_player_id={
                            let id = player_id.get();
                            if id.to_string().is_empty() { None } else { Some(id) }
                        }
                        on_left=Callback::new(move |_| {
                            handle_leave_and_cleanup();
                        })
                        lobby_info=lobby_info.read_only()
                    />
                    </Show>
                }
            }
        >
            <div class="text-center p-8">
                <div class="text-lg text-gray-600 dark:text-gray-300">"Loading..."</div>
            </div>
        </Show>
    }
}
