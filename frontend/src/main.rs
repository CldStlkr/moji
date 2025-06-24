// frontend/src/main.rs
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod api;
mod components;
mod error;
mod persistence;

use components::game::GameComponent;
use components::lobby::LobbyComponent;
use persistence::{clear_session, load_session, use_session_persistence};
use shared::PlayerId;
use wasm_bindgen_futures::spawn_local;

#[component]
fn App() -> impl IntoView {
    let (lobby_id, set_lobby_id) = signal(String::new());
    let (player_id, set_player_id) = signal(PlayerId::default());
    let (player_name, set_player_name) = signal(String::new());
    let (is_in_game, set_is_in_game) = signal(false);
    let (is_restoring, set_is_restoring) = signal(true);

    // Set up session persistence
    use_session_persistence(lobby_id, player_id, player_name, is_in_game);

    // Check for existing session on mount
    Effect::new(move |_| {
        spawn_local(async move {
            // Try to restore session
            if let Some(session_data) = load_session() {
                // Validate the session is still valid by checking with the server
                match api::get_player_info(&session_data.lobby_id, &session_data.player_id).await {
                    Ok(player_info) => {
                        // Session is valid, restore it
                        set_lobby_id.set(session_data.lobby_id);
                        set_player_id.set(session_data.player_id);
                        set_player_name.set(player_info.name);
                        set_is_in_game.set(session_data.is_in_game);
                    }
                    Err(_) => {
                        // Session is invalid, clear it
                        clear_session();
                    }
                }
            }
            set_is_restoring.set(false);
        });
    });

    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        set_lobby_id.set(new_lobby_id);
        set_player_id.set(new_player_id);
        set_is_in_game.set(true);
    };

    let handle_exit_game = move || {
        set_is_in_game.set(false);
        set_lobby_id.set(String::new());
        set_player_id.set(PlayerId::default());
        set_player_name.set(String::new());
        clear_session();
    };

    view! {
        <div class="max-w-4xl mx-auto p-5">
            <header class="text-center mb-8">
                <h1 class="text-4xl font-bold text-blue-500">"文字"</h1>
            </header>
            <main>
                <Show
                    when=move || is_restoring.get()
                    fallback=move || view! {
                        <Show
                            when=move || !is_in_game.get()
                            fallback=move || view! {
                                <GameComponent
                                    lobby_id=lobby_id.get()
                                    player_id=player_id.get()
                                    on_exit_game=handle_exit_game
                                />
                            }
                        >
                            <LobbyComponent on_lobby_joined=handle_lobby_joined />
                        </Show>
                    }
                >
                    <div class="text-center p-8">
                        <div class="text-lg text-gray-600">"Loading..."</div>
                    </div>
                </Show>
            </main>
            <footer class="text-center mt-8 pt-4 border-t border-gray-200 text-gray-600 text-sm">
                <p>"Learn Japanese Kanji through word recognition"</p>
            </footer>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {}
