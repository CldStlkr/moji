use leptos::{prelude::*};
use leptos_router::{StaticSegment, ParamSegment, NavigateOptions, hooks::{use_params_map, use_navigate }, components::{Route, Router, Routes} };


use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use moji_frontend::api;
use moji_frontend::components;
use moji_frontend::persistence;

use components::game::GameComponent;
use components::lobby::{ LobbyComponent, LobbyOrJoinComponent };
use persistence::{clear_session, load_session, use_session_persistence};
use shared::PlayerId;

#[component]
fn App() -> impl IntoView {
    view! {
      <Router>
        <div class="p-5 mx-auto max-w-4xl">
          <header class="mb-8 text-center">
            <h1 class="text-4xl font-bold text-blue-500">"文字"</h1>
          </header>
          <main>
            <Routes fallback=|| view! { <div>"Page not found"</div> }>
              <Route path=() view=HomePage />
              <Route path=(StaticSegment("lobby"), ParamSegment("lobby_id")) view=LobbyRoute />
            </Routes>
          </main>
          <footer class="pt-4 mt-8 text-sm text-center text-gray-600 border-t border-gray-200">
            <p>"Learn Japanese Kanji through word recognition"</p>
          </footer>
        </div>
      </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let navigate = StoredValue::new(use_navigate());
    let lobby_id = RwSignal::new(String::new());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let is_in_game = RwSignal::new(false);
    let is_restoring = RwSignal::new(true);


    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        leptos::logging::log!("NAVIGATION: Moving to /lobby/{}", new_lobby_id);
        lobby_id.set(new_lobby_id.clone());
        player_id.set(new_player_id);
        is_in_game.set(false);
        // Navigate to the lobby
        navigate.with_value(|nav| {
            nav(&format!("/lobby/{}", new_lobby_id), NavigateOptions::default());
        });
    };

    use_session_persistence(
        lobby_id.read_only(),
        player_id.read_only(),
        player_name.read_only(),
        is_in_game.read_only(),
    );

    // Check for existing session on mount
    Effect::new(move |_| {
        spawn_local(async move {
            // Try to restore session
            if let Some(session_data) = load_session() {
                // Validate the session with the server
                match api::get_player_info(&session_data.lobby_id, &session_data.player_id).await {
                    Ok(player_info) => {
                        lobby_id.set(session_data.lobby_id.clone());
                        player_id.set(session_data.player_id.clone());
                        player_name.set(player_info.name);
                        is_in_game.set(session_data.is_in_game);
                        
                        // Navigate to the lobby
                        navigate.with_value(|nav| {
                            nav(&format!("/lobby/{}", session_data.lobby_id), NavigateOptions::default());
                        });
                        
                    }
                    Err(_) => {
                        clear_session();
                    }
                }
            }
            is_restoring.set(false);
        });
    });

        

    view! {
      <Show
        when=move || is_restoring.get()
        fallback=move || {
          view! { <LobbyComponent on_lobby_joined=handle_lobby_joined /> }
        }
      >
        <div class="p-8 text-center">
          <div class="text-lg text-gray-600">"Loading..."</div>
        </div>
      </Show>
    }
}

#[component]
fn LobbyRoute() -> impl IntoView {
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let lobby_id_from_url = move || {
        params.with(|p| p.get("lobby_id").unwrap_or_default())
    };

    let lobby_id = RwSignal::new(lobby_id_from_url());
    let player_id = RwSignal::new(PlayerId::default());
    let player_name = RwSignal::new(String::new());
    let is_in_game = RwSignal::new(false);
    let is_restoring = RwSignal::new(true);

    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        lobby_id.set(new_lobby_id);
        player_id.set(new_player_id);
        is_in_game.set(false);
    };
    let handle_exit_lobby = move || {
        lobby_id.set(String::new());
        player_id.set(PlayerId::default());
        player_name.set(String::new());
        is_in_game.set(false);
        clear_session();
    };

    let handle_game_started = move || {
        is_in_game.set(true);
    };
    let handle_exit_game = move ||  {
        is_in_game.set(false);
        lobby_id.set(String::new());
        player_id.set(PlayerId::default());
        player_name.set(String::new());
        clear_session();

        navigate.with_value(|nav| nav("/", NavigateOptions::default()));
    };

    use_session_persistence(
        lobby_id.read_only(),
        player_id.read_only(),
        player_name.read_only(),
        is_in_game.read_only(),
    );

    // Check session and validate against URL
    Effect::new(move |_| {
        let url_lobby_id = lobby_id_from_url();
        
        spawn_local(async move {
            if let Some(session_data) = load_session() {
                // If session lobby matches URL, restore session
                if session_data.lobby_id == url_lobby_id {
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
                } else {
                    // URL and session don't match - clear session and treat as new join
                    clear_session();
                    lobby_id.set(url_lobby_id);
                }
            } else {
                // No session - treat as new join attempt
                lobby_id.set(url_lobby_id);
            }
            is_restoring.set(false);
        });
    });


    view! {
      <Show
        when=move || is_restoring.get()
        fallback=move || {
          view! {
            <Show
              when=move || is_in_game.get()
              fallback=move || {
                view! {
                  <LobbyOrJoinComponent
                    lobby_id=lobby_id.read_only()
                    player_id=player_id.read_only()
                    player_name=player_name.read_only()
                    on_lobby_joined=handle_lobby_joined
                    on_game_started=handle_game_started
                    on_exit_lobby=handle_exit_game
                  />
                }
              }
            >
              <GameComponent
                lobby_id=lobby_id.get()
                player_id=player_id.get()
                on_exit_game=handle_exit_game
              />
            </Show>
          }
        }
      >
        <div class="p-8 text-center">
          <div class="text-lg text-gray-600">"Loading..."</div>
        </div>
      </Show>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {}
