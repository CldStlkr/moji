use leptos::prelude::*;
use shared::PlayerId;
use wasm_bindgen_futures::spawn_local;

use crate::{
    api,
    components::{game::GameComponent, lobby::LobbyComponent},
    persistence::{clear_session, load_session, use_session_persistence},
};

#[component]
pub fn Home() -> impl IntoView {
    let lobby_id = RwSignal::new(String::new());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let is_in_game = RwSignal::new(false);
    let is_restoring = RwSignal::new(true);

    use_session_persistence(
        lobby_id.read_only(),
        player_id.read_only(),
        player_name.read_only(),
        is_in_game.read_only(),
    );

    // Check for existing session on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Some(session_data) = load_session() {
                match api::get_player_info(&session_data.lobby_id, &session_data.player_id).await {
                    Ok(player_info) => {
                        lobby_id.set(session_data.lobby_id);
                        player_id.set(session_data.player_id);
                        player_name.set(player_info.name);
                        is_in_game.set(session_data.is_in_game);
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
        is_in_game.set(true);
        navigate_path.set(Some(format!("/join/{}", new_lobby_id)));
    };

    let handle_exit_game = move || {
        is_in_game.set(false);
        lobby_id.set(String::new());
        player_id.set(PlayerId::default());
        player_name.set(String::new());
        clear_session();
        navigate_path.set(Some("/".to_string()));
    };

    let handle_return_to_lobby = move || {
        is_in_game.set(false);
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
                                    lobby_id=lobby_id.get()
                                    player_id=player_id.get()
                                    on_exit_game=handle_exit_game
                                    on_return_to_lobby=handle_return_to_lobby
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
                                navigate_path.set(Some("/".to_string()));
                            })
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
