use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use moji_frontend::api;
use moji_frontend::components;


use components::auth_modal::AuthModal;
use components::game::GameComponent;
use components::lobby::LobbyComponent;
use components::user_menu::UserMenu;
use moji_frontend::context::{AuthContext, User};
use moji_frontend::persistence::{clear_session, load_session, load_auth, use_session_persistence};
use shared::PlayerId;
use wasm_bindgen_futures::spawn_local;

#[component]
fn App() -> impl IntoView {
    let lobby_id = RwSignal::new(String::new());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let is_in_game = RwSignal::new(false);
    let is_restoring = RwSignal::new(true);

    // Auth Context State
    let (user, set_user) = signal::<Option<User>>(None);
    let (show_auth_modal, set_show_auth_modal) = signal(false);
    
    provide_context(AuthContext {
        user,
        set_user,
        show_auth_modal,
        set_show_auth_modal,
    });

    use_session_persistence(
        lobby_id.read_only(),
        player_id.read_only(),
        player_name.read_only(),
        is_in_game.read_only(),
    );

    // Check for existing session & auth on mount
    Effect::new(move |_| {
        // Load Auth
        if let Some(auth) = load_auth() {
             set_user.set(Some(User {
                 username: auth.username.clone(),
                 is_guest: auth.is_guest,
             }));
        }

        spawn_local(async move {
            // Try to restore session
            if let Some(session_data) = load_session() {
                // Validate the session is still valid by checking with the server
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

    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        lobby_id.set(new_lobby_id);
        player_id.set(new_player_id);
        is_in_game.set(true);
    };

    let handle_exit_game = move || {
        is_in_game.set(false);
        lobby_id.set(String::new());
        player_id.set(PlayerId::default());
        player_name.set(String::new());
        clear_session();
    };

    let is_dark_mode = RwSignal::new(false);

    // Initialize dark mode from local storage
    Effect::new(move |_| {
        if let Ok(Some(storage)) = window().local_storage() {
            if let Ok(Some(value)) = storage.get_item("dark_mode") {
                is_dark_mode.set(value == "true");
            }
        }
    });

    // Toggle dark mode class on html element
    Effect::new(move |_| {
        let is_dark = is_dark_mode.get();
        let doc = web_sys::window().unwrap().document().unwrap().document_element().unwrap();
        if is_dark {
            let _ = doc.class_list().add_1("dark");
        } else {
            let _ = doc.class_list().remove_1("dark");
        }
        
        if let Ok(Some(storage)) = window().local_storage() {
            let _ = storage.set_item("dark_mode", if is_dark { "true" } else { "false" });
        }
    });

    view! {
        <div class="max-w-4xl mx-auto p-5 dark:text-gray-100">
            <header class="flex justify-between items-center mb-8">
                <h1 class="text-4xl font-bold text-blue-500">"文字"</h1>
                <div class="flex items-center space-x-4">
                     <UserMenu />
                     <button
                        on:click=move |_| is_dark_mode.update(|d| *d = !*d)
                        class="p-2 rounded-full hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                        title="Toggle Dark Mode"
                    >
                        {move || if is_dark_mode.get() { "🌙" } else { "☀️" }}
                    </button>
                </div>
            </header>
            <main>
                <Show when=move || show_auth_modal.get()>
                    <AuthModal
                        on_close=Callback::from(move || set_show_auth_modal.set(false)) 
                        on_success=Callback::from(move || set_show_auth_modal.set(false))
                    />
                </Show>

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
                                        />
                                    }
                                }
                            >
                                <LobbyComponent on_lobby_joined=handle_lobby_joined />
                            </Show>
                        }
                    }
                >
                    <div class="text-center p-8">
                        <div class="text-lg text-gray-600 dark:text-gray-300">"Loading..."</div>
                    </div>
                </Show>
            </main>
            <footer class="text-center mt-8 pt-4 border-t border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm">
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
