use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod api;
mod components;
mod error;
use components::game::GameComponent;
use components::lobby::LobbyComponent;
use shared::PlayerId;

#[component]
fn App() -> impl IntoView {
    let (lobby_id, set_lobby_id) = signal(String::new());
    let (player_id, set_player_id) = signal(PlayerId::default()); // Added player_id signal
    let (is_in_game, set_is_in_game) = signal(false);

    // Updated to handle both lobby_id and player_id
    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        set_lobby_id.set(new_lobby_id);
        set_player_id.set(new_player_id);
        set_is_in_game.set(true);
    };

    let handle_exit_game = move || {
        set_is_in_game.set(false);
        set_lobby_id.set(String::new());
        set_player_id.set(PlayerId::default());
    };

    view! {
        <div class="app-container">
            <header>
                <h1>"文字"</h1>
            </header>
            <main>
                <Show
                    when=move || !is_in_game.get()
                    fallback=move || view! {
                        <GameComponent
                            lobby_id=lobby_id.get()
                            player_id=player_id.get() // Pass player_id to GameComponent
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
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {}
