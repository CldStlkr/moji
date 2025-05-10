use leptos::mount::mount_to_body;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod api;
mod components;

use components::game::GameComponent;
use components::lobby::LobbyComponent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanjiPrompt {
    pub kanji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInput {
    pub word: String,
    pub kanji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub lobby_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckWordResponse {
    pub message: String,
    pub score: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[component]
fn App() -> impl IntoView {
    let (lobby_id, set_lobby_id) = signal(String::new());
    let (is_in_game, set_is_in_game) = signal(false);

    let handle_lobby_joined = move |new_lobby_id: String| {
        set_lobby_id.set(new_lobby_id);
        set_is_in_game.set(true);
    };

    let handle_exit_game = move || {
        set_is_in_game.set(false);
        set_lobby_id.set(String::new());
    };

    view! {
        <div class="app-container">
            <header>
                <h1>"Kanji Guessing Game"</h1>
            </header>
            <main>
                <Show
                    when=move || !is_in_game.get()
                    fallback=move || view! {
                        <GameComponent
                            lobby_id=lobby_id.get()
                            on_exit_game=handle_exit_game
                        />
                    }
                >
                    <LobbyComponent on_lobby_joined=handle_lobby_joined />
                </Show>
            </main>
            <footer>
                <p>"Learn Japanese Kanji through word recognition"</p>
            </footer>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
