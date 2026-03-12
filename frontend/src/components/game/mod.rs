use crate::components::player_scores::CompactPlayerScoresComponent;
use crate::styled_view;
use crate::error::get_user_friendly_message;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::{LobbyId, PlayerId, ClientMessage, reset_lobby};
use wasm_bindgen_futures::spawn_local;

mod header;
mod prompt;
mod input;
mod feedback;
mod timer;
mod game_over;

use header::GameHeader;
use prompt::PromptDisplay;
use input::GameInput;
use feedback::GameFeedback;
use game_over::GameOver;
use timer::TimerBar;

styled_view!(game_container, "max-w-6xl mx-auto my-8 p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors");
styled_view!(lobby_info_bar, "flex items-center gap-2 mb-6 p-2 bg-gray-100 dark:bg-gray-700 rounded text-sm relative transition-colors");
styled_view!(lobby_id_label, "text-gray-700 dark:text-gray-300");
styled_view!(lobby_id_value, "font-bold tracking-wider text-blue-600 dark:text-blue-400");
styled_view!(copy_btn, "ml-2 px-1 py-0.5 text-xs font-medium bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded transition-all duration-200 hover:bg-blue-50 dark:hover:bg-gray-500 hover:border-blue-400 hover:shadow-sm active:scale-95 active:bg-blue-100 dark:text-gray-200");
styled_view!(copied_msg, "absolute -top-8 left-1/2 transform -translate-x-1/2 px-2 py-1 bg-green-500 text-white text-xs rounded shadow-lg animate-fade-in pointer-events-none");

#[component]
pub fn GameComponent<F, M>(
    lobby_id: ReadSignal<LobbyId>,
    player_id: ReadSignal<PlayerId>,
    on_exit_game: F,
    send_message: M,
    prompt: ReadSignal<String>,
    result: ReadSignal<String>,
    typing_status: RwSignal<std::collections::HashMap<PlayerId, String>>,
    lobby_info: ReadSignal<Option<shared::LobbyInfo>>,
) -> impl IntoView
where
    F: Fn() + 'static + Copy + Send + Sync,
    M: Fn(ClientMessage) + Copy + 'static,
{
    // We don't need to instantiate rw signals for props we are receiving.
    let word = RwSignal::new(String::new());
    // These specific piece of state could arguably be lifted up, but Game Component can manage them for now.
    let is_loading = RwSignal::new(false);
    let is_copied = RwSignal::new(false);
    let error_message = RwSignal::new(String::new());
    let shake_trigger = RwSignal::new(false);

    // Watch for result changes to trigger shake
    Effect::new(move |_| {
        let res = result.get();
        if !res.is_empty() && (res.contains("Invalid") || res.contains("Wrong") || res.contains("Try")) {
            shake_trigger.set(true);
            set_timeout(move || shake_trigger.set(false), std::time::Duration::from_millis(500));
        }
    });

    let input_ref = NodeRef::<html::Input>::new();

    let perform_submit = move || {
        let current_word = word.get();
        let current_prompt = prompt.get();

        if current_word.trim().is_empty() { return; }

        let msg = ClientMessage::Submit {
            input: current_word,
            prompt: current_prompt
        };
        send_message(msg);

        // Clear input immediately
        word.set("".to_string());
        if let Some(input) = input_ref.get() {
            input.set_value("");
            let _ = input.focus();
        }
    };

    let handle_input = move |ev| {
        let val = event_target_value(&ev);
        word.set(val.clone());

        let msg = ClientMessage::Typing { input: val };
        send_message(msg);
    };


    // Focus input on mount
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    let submit_word = move |_: ev::MouseEvent| {
        perform_submit();
    };

    let skip_turn = move |_: ev::MouseEvent| {
        let msg = ClientMessage::Skip;
        send_message(msg);
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        let is_my_turn = if let Some(info) = lobby_info.get() {
            if info.settings.mode == shared::GameMode::Duel {
                info.players.iter().find(|p| p.id == player_id.get()).map(|p| p.is_turn).unwrap_or(false)
            } else {
                true
            }
        } else {
            true
        };

        if ev.key() == "Enter" && !is_loading.get() && is_my_turn {
            perform_submit();
        }
    };

    let handle_exit_game = move || {
        // Cleanup (leave_lobby API call, state reset, nav) is handled by
        // Home::handle_leave_and_cleanup which is passed as on_exit_game.
        on_exit_game();
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let id_str = lobby_id.get().to_string();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&id_str)).await {
                Ok(_) => {
                    is_copied.set(true);

                    set_timeout(
                        move || {
                            is_copied.set(false);
                        },
                        std::time::Duration::from_millis(1000),
                    );
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard")
                }
            }
        });
    };

    let handle_return_to_lobby = move || {
        let lid = lobby_id.get();
        let pid = player_id.get();

        spawn_local(async move {
            if let Err(e) = reset_lobby(lid, pid).await {
                error_message.set(get_user_friendly_message(e));
            }
        });
    };

    let player_name = Signal::derive(move || {
        lobby_info.get()
            .and_then(|info| info.players.into_iter().find(|p| p.id == player_id.get()))
            .map(|p| p.name)
            .unwrap_or_else(|| "Unknown".to_string())
    });

    let score = Signal::derive(move || {
        lobby_info.get()
            .and_then(|info| info.players.into_iter().find(|p| p.id == player_id.get()))
            .map(|p| p.score)
            .unwrap_or_default()
    });

    view! {
        <div class=move || format!("{} {}", game_container(), if shake_trigger.get() { "animate-shake" } else { "" })>

            <GameHeader
                content_mode=lobby_info.get().map(|i| i.settings.content_mode).unwrap_or_default()
                player_name=player_name
                score=score
                on_exit=handle_exit_game
            />

            // Lobby Info
            <div class=lobby_info_bar()>
                <span class=lobby_id_label()>"Lobby ID:"</span>
                <span class=lobby_id_value()>{move || lobby_id.get().to_string()}</span>
                <button
                    on:click=copy_lobby_id
                    class=copy_btn()
                    title="Copy Lobby ID"
                >
                    "Copy"
                </button>

                // Floating "Copied!" text using Show
                <Show when=move || is_copied.get()>
                    <div class=copied_msg()>
                        "Copied!"
                    </div>
                </Show>
            </div>

            // Game Layout with Sidebar
            <div class="flex gap-8 flex-col lg:flex-row">
                // Main Game Area
                <div class="flex-1 space-y-8">

                    <TimerBar
                        time_limit=Signal::derive(move || -> Option<u32> {
                            lobby_info.get().and_then(|info| {
                                if info.settings.mode != shared::GameMode::Zen {
                                    info.settings.time_limit_seconds
                                } else {
                                    None
                                }
                            })
                        })
                        reset_trigger=Signal::derive(move || -> String { prompt.get() })
                    />

                    <PromptDisplay
                        prompt=prompt
                        is_loading=is_loading.read_only()
                    />

                    <GameInput
                        content_mode=lobby_info.get().map(|i| i.settings.content_mode).unwrap_or_default()
                        input_ref=input_ref
                        word=word.read_only()
                        prompt=prompt
                        is_loading=is_loading.read_only()
                        on_input=handle_input
                        on_submit=submit_word
                        on_keydown=handle_key_press
                        on_skip=skip_turn
                        disabled=Signal::derive(move || {
                             if let Some(info) = lobby_info.get() {
                                 if info.settings.mode == shared::GameMode::Duel {
                                     let me = info.players.into_iter().find(|p| p.id == player_id.get());
                                     !me.map(|p| p.is_turn).unwrap_or(false)
                                 } else {
                                     false
                                 }
                             } else {
                                 false
                             }
                        })
                    />

                    <GameFeedback
                        content_mode=lobby_info.get().map(|i| i.settings.content_mode).unwrap_or_default()
                        result=result
                        error_message=error_message.read_only()
                    />

                </div>

                // Scores Sidebar
                <div class="w-full lg:w-64 flex-shrink-0">
                    <Show when=move || lobby_info.get().map(|i| !i.players.is_empty()).unwrap_or(false)>
                        <CompactPlayerScoresComponent
                            players=Signal::derive(move || lobby_info.get().map(|i| i.players).unwrap_or_default()).get()
                            current_player_id=player_id
                            typing_status=typing_status.read_only()
                            game_mode=Signal::derive(move || lobby_info.get().map(|i| i.settings.mode).unwrap_or_default())
                            initial_lives=Signal::derive(move || lobby_info.get().map(|i| i.settings.initial_lives.unwrap_or(3)).unwrap_or(3))
                        />
                    </Show>
                </div>
            </div>

            <Show when=move || lobby_info.get().map(|i| i.status == shared::GameStatus::Finished).unwrap_or(false)>
                <GameOver
                    players=Signal::derive(move || lobby_info.get().map(|i| i.players).unwrap_or_default()).get()
                    mode=Signal::derive(move || lobby_info.get().map(|i| i.settings.mode).unwrap_or_default()).get()
                    current_player_id=player_id.get()
                    is_leader=Signal::derive(move || {
                        if let Some(info) = lobby_info.get() {
                            let lid = info.leader_id;
                            let pid = player_id.get();
                            !lid.to_string().is_empty() && lid == pid
                        } else {
                            false
                        }
                    })
                    on_return_to_lobby=handle_return_to_lobby
                    on_exit=handle_exit_game
                />
            </Show>
        </div>
    }
}
