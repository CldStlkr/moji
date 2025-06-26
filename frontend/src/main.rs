use leptos::prelude::*;
use wasm_bindgen::prelude::*;
mod api;
mod app_state;
mod components;
mod error;
mod persistence;
use app_state::AppState;
use components::game::GameComponent;
use components::lobby::LobbyComponent;
use persistence::{clear_session, load_session, use_session_persistence};
use shared::PlayerId;
use wasm_bindgen_futures::spawn_local;

#[component]
fn App() -> impl IntoView {
    // All signals created at component level for proper lifecycle management
    let (lobby_id, set_lobby_id) = signal(String::new());
    let (player_id, set_player_id) = signal(PlayerId::default());
    let (player_name, set_player_name) = signal(String::new());
    let (app_state, set_app_state) = signal(AppState::NotInLobby);
    let (is_restoring, set_is_restoring) = signal(true);

    // Set up session persistence
    use_session_persistence(lobby_id, player_id, player_name, app_state);

    // Check for existing session on mount
    Effect::new(move |_| {
        spawn_local(async move {
            web_sys::console::log_1(&"Session restoration starting...".into());

            // Try to restore session
            if let Some(session_data) = load_session() {
                web_sys::console::log_1(
                    &format!(
                        "Found session: lobby_id={}, app_state={:?}",
                        session_data.lobby_id, session_data.app_state
                    )
                    .into(),
                );

                // First try to get lobby info to see if the lobby still exists
                match api::get_lobby_info(&session_data.lobby_id).await {
                    Ok(lobby_info) => {
                        web_sys::console::log_1(
                            &format!("Lobby still exists, status: {:?}", lobby_info.status).into(),
                        );

                        // Restore session data
                        set_lobby_id.set(session_data.lobby_id.clone());
                        set_player_id.set(session_data.player_id.clone());
                        set_player_name.set(session_data.player_name.clone());

                        // Use lobby status to determine state
                        match lobby_info.status {
                            shared::GameStatus::Lobby => {
                                web_sys::console::log_1(&"Restoring to InLobby state".into());
                                set_app_state.set(AppState::InLobby);
                            }
                            shared::GameStatus::Playing | shared::GameStatus::Finished => {
                                web_sys::console::log_1(&"Restoring to InGame state".into());
                                set_app_state.set(AppState::InGame);
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::log_1(
                            &format!("Lobby no longer exists: {:?} - clearing session", e).into(),
                        );
                        // Lobby doesn't exist anymore, clear session
                        clear_session();
                    }
                }
            } else {
                web_sys::console::log_1(&"No session found".into());
            }

            set_is_restoring.set(false);
        });
    });

    // Handle initial lobby join (from create/join screen)
    let handle_lobby_joined = move |new_lobby_id: String, new_player_id: PlayerId| {
        web_sys::console::log_1(
            &format!("handle_lobby_joined: {} {}", new_lobby_id, new_player_id.0).into(),
        );

        // Set the IDs immediately
        set_lobby_id.set(new_lobby_id.clone());
        set_player_id.set(new_player_id.clone());

        // Transition to lobby state immediately
        set_app_state.set(AppState::InLobby);

        // Then fetch player name asynchronously
        let lid = new_lobby_id;
        let pid = new_player_id;
        spawn_local(async move {
            match api::get_player_info(&lid, &pid).await {
                Ok(player_info) => {
                    web_sys::console::log_1(
                        &format!("Got player name: {}", player_info.name).into(),
                    );
                    set_player_name.set(player_info.name);
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to get player info: {:?}", e).into(),
                    );
                }
            }
        });
    };

    // Handle game start (from lobby to game)
    let handle_game_started = move || {
        web_sys::console::log_1(&"handle_game_started called - setting state to InGame".into());
        set_app_state.set(AppState::InGame);
    };

    // Handle complete exit (leave lobby entirely)
    let handle_exit_game = move || {
        web_sys::console::log_1(&"handle_exit_game called - resetting to NotInLobby".into());
        set_app_state.set(AppState::NotInLobby);
        set_lobby_id.set(String::new());
        set_player_id.set(PlayerId::default());
        set_player_name.set(String::new());
        clear_session();
    };

    // Handle return to lobby (from game back to pre-game lobby)
    let handle_return_to_lobby = move || {
        set_app_state.set(AppState::InLobby);
    };

    view! {
        <div class="max-w-4xl mx-auto p-5">
            <header class="text-center mb-8">
                <h1 class="text-4xl font-bold text-blue-500">"文字"</h1>
            </header>
            <main>
                <Show
                    when=move || is_restoring.get()
                    fallback=move || {
                        view! {
                            <AppContent
                                app_state=app_state
                                lobby_id=lobby_id
                                player_id=player_id
                                player_name=player_name
                                set_player_name=set_player_name
                                on_lobby_joined=handle_lobby_joined
                                on_game_started=handle_game_started
                                on_exit_game=handle_exit_game
                                on_return_to_lobby=handle_return_to_lobby
                            />
                        }
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

#[component]
fn AppContent<F1, F2, F3, F4>(
    app_state: ReadSignal<AppState>,
    lobby_id: ReadSignal<String>,
    player_id: ReadSignal<PlayerId>,
    player_name: ReadSignal<String>,
    set_player_name: WriteSignal<String>,
    on_lobby_joined: F1,
    on_game_started: F2,
    on_exit_game: F3,
    on_return_to_lobby: F4,
) -> AnyView
where
    F1: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
    F3: Fn() + 'static + Copy + Send + Sync,
    F4: Fn() + 'static + Copy + Send + Sync,
{
    match app_state.get() {
        AppState::NotInLobby => view! {
            <LobbyComponent on_lobby_joined=on_lobby_joined />
        }
        .into_any(),

        AppState::InLobby => view! {
            <InLobbyView
                lobby_id=lobby_id
                player_id=player_id
                on_game_started=on_game_started
                on_exit_game=on_exit_game
            />
        }
        .into_any(),

        AppState::InGame => view! {
            <GameComponent
                lobby_id=lobby_id.get()
                player_id=player_id.get()
                on_exit_game=on_exit_game
                on_return_to_lobby=on_return_to_lobby
            />
        }
        .into_any(),
    }
}

#[component]
fn InLobbyView<F1, F2>(
    lobby_id: ReadSignal<String>,
    player_id: ReadSignal<PlayerId>,
    on_game_started: F1,
    on_exit_game: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    // Create lobby-specific signals at proper component level
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (in_lobby, set_in_lobby) = signal(true);
    let (lobby_info, set_lobby_info) = signal(None);

    // Create a version tracking signal to detect changes
    let (last_status, set_last_status) = signal(shared::GameStatus::Lobby);

    // Custom polling that properly detects game start
    Effect::new(move |_| {
        let lobby_id_val = lobby_id.get();
        let in_lobby_val = in_lobby.get();

        web_sys::console::log_1(
            &format!(
                "InLobbyView Effect: lobby_id={}, in_lobby={}",
                lobby_id_val, in_lobby_val
            )
            .into(),
        );

        if !lobby_id_val.is_empty() && in_lobby_val {
            spawn_local(async move {
                // Initial delay to let component settle
                gloo_timers::future::TimeoutFuture::new(500).await;

                loop {
                    if !in_lobby.get() {
                        break;
                    }

                    match api::get_lobby_info(&lobby_id_val).await {
                        Ok(info) => {
                            let current_status = info.status;
                            let previous_status = last_status.get();

                            web_sys::console::log_1(
                                &format!("Lobby status: {:?}", current_status).into(),
                            );

                            // Detect transition from Lobby to Playing
                            if previous_status == shared::GameStatus::Lobby
                                && current_status == shared::GameStatus::Playing
                            {
                                web_sys::console::log_1(&"Game started! Transitioning...".into());
                                set_in_lobby.set(false);
                                on_game_started();
                                break;
                            }

                            set_last_status.set(current_status);
                            set_lobby_info.set(Some(info));
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("Polling error: {:?}", e).into());
                        }
                    }

                    // Wait before next poll
                    gloo_timers::future::TimeoutFuture::new(1000).await;
                }
            });
        }
    });

    // Handle leaving the lobby
    let handle_leave_lobby = move |_| {
        web_sys::console::log_1(&"InLobbyView: leave lobby called".into());
        set_in_lobby.set(false);
        on_exit_game();
    };

    // Clean up polling when component unmounts
    on_cleanup(move || {
        set_in_lobby.set(false);
    });

    view! {
        <div class="max-w-2xl mx-auto my-8">
            <components::lobby::lobby_management::LobbyManagementComponent
                lobby_info=lobby_info
                current_lobby_id=lobby_id
                current_player_id=player_id
                _is_loading=is_loading
                set_is_loading=set_is_loading
                _status=status
                set_status=set_status
                on_leave_lobby=handle_leave_lobby
            />
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {}
