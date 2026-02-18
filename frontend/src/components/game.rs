use crate::api;
use crate::components::player_scores::CompactPlayerScoresComponent;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::{PlayerData, PlayerId, ClientMessage, ServerMessage};
use wasm_bindgen_futures::spawn_local;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};

#[component]
pub fn GameComponent<F>(lobby_id: String, player_id: PlayerId, on_exit_game: F) -> impl IntoView
where
    F: Fn() + 'static + Copy,
{
    let kanji = RwSignal::new(String::new());
    let word = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    let score = RwSignal::new(0u32);
    let player_name = RwSignal::new(String::new());
    let is_loading = RwSignal::new(false);
    let is_copied = RwSignal::new(false);
    let error_message = RwSignal::new(String::new());
    let all_players = RwSignal::<Vec<PlayerData>>::new(Vec::new()); 
    let typing_status = RwSignal::new(std::collections::HashMap::<PlayerId, String>::new());

    let input_ref = NodeRef::<html::Input>::new();

    // Store signals for use in async contexts
    let (lobby_id_signal, _) = signal(lobby_id.clone());
    let (player_id_signal, _) = signal(player_id.clone());

    // Signal to hold the current WebSocket sender
    let ws_sender = RwSignal::new(None::<futures::channel::mpsc::UnboundedSender<String>>);

    Effect::new(move |_| {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();

        // Create a FRESH channel for this connection attempt
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
        
        // Store the sender so perform_submit can use it
        ws_sender.set(Some(tx));

        spawn_local(async move {
            // Calculate WS URL
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
            let host = location.host().unwrap();
            let ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);

            let ws = match WebSocket::open(&ws_url) {
                Ok(ws) => ws,
                Err(e) => {
                    leptos::logging::error!("Failed to open connection: {:?}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws.split();

            // First, forward outgoing messages (channel -> WebSocket)
            spawn_local(async move {
                while let Some(msg) = rx.next().await {
                    let _ = write.send(Message::Text(msg)).await;
                }
            });

            // Second, handle incoming messages (WebSocket -> Signals)
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        match server_msg {
                            ServerMessage::GameState { kanji: new_kanji, status: _, scores } => {
                                kanji.set(new_kanji);
                                all_players.set(scores);
                                typing_status.update(|m| m.clear());
                            },
                            ServerMessage::WordChecked { player_id: pid, result: res } => {
                                // Show result if it's our submission
                                if pid == player_id {
                                    result.set(res.message);
                                    score.set(res.score);
                                    if let Some(k) = res.kanji {
                                        kanji.set(k);
                                    }
                                }
                                // If a word was checked (even if wrong), maybe we don't clear typing?
                                // Actually, if it was wrong, they might keep typing.
                                // If it was right (new kanji), that comes via KanjiUpdate usually.
                            },
                            ServerMessage::KanjiUpdate { new_kanji } => {
                                // Clear old result when new kanji arrives
                                result.set(String::new());
                                kanji.set(new_kanji);
                                typing_status.update(|m| m.clear());
                            },
                            ServerMessage::PlayerListUpdate { players } => {
                                all_players.set(players.clone());
                                // Update own score locally
                                if let Some(me) = players.iter().find(|p| p.id == player_id) {
                                    score.set(me.score);
                                }
                            },
                            ServerMessage::PlayerTyping { player_id: pid, input } => {
                                typing_status.update(|m| {
                                    if input.is_empty() {
                                        m.remove(&pid);
                                    } else {
                                        m.insert(pid, input);
                                    }
                                });
                            }
                            _ => {},
                        }
                    }
                }
            }

        });
    });

    let perform_submit = move || {
        let current_word = word.get();
        let current_kanji = kanji.get();
        let _lobby_id = lobby_id_signal.get();
        let _player_id = player_id_signal.get();

        if current_word.trim().is_empty() { return; }

        let msg = ClientMessage::Submit {
            word: current_word,
            kanji: current_kanji
        };

        // Send to writer task
        if let Some(mut sender) = ws_sender.get_untracked() {
            let payload = serde_json::to_string(&msg).unwrap();
            spawn_local(async move { let _ = sender.send(payload).await; });
        }

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

        if let Some(mut sender) = ws_sender.get_untracked() {
             let msg = ClientMessage::Typing { input: val };
             // Use unwrap_or_default or handle error if needed, but here simple unwrap is fine mostly
             // or just ignore errors
             if let Ok(payload) = serde_json::to_string(&msg) {
                 spawn_local(async move {
                     let _ = sender.send(payload).await;
                 });
             }
        }
    };

    Effect::new(move |_| {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();
        let input_ref = input_ref;

        spawn_local(async move {
            is_loading.set(true);
            error_message.set(String::new());
            result.set(String::new());

            match api::get_player_info(&lobby_id, &player_id).await {
                Ok(info) => {
                    player_name.set(info.name);
                    score.set(info.score);
                }
                Err(e) => {
                    error_message.set(format!("Could not fetch player info: {}", e));
                }
            }

            match api::get_kanji(&lobby_id).await {
                Ok(prompt) => {
                    kanji.set(prompt.kanji);
                }
                Err(e) => {
                    error_message.set(format!("Could not fetch kanji: {}", e));
                }
            }

            is_loading.set(false);

            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        });
    });

    on_cleanup(move || {
    });

    let submit_word = move |_: ev::MouseEvent| {
        perform_submit();
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            perform_submit();
        }
    };

    let handle_exit_game = move |_: ev::MouseEvent| {
        on_exit_game();
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = lobby_id_signal.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await {
                Ok(_) => {
                    is_copied.set(true);

                    gloo_timers::future::TimeoutFuture::new(1000).await;
                    is_copied.set(false);
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard")
                }
            }
        });
    };

    let get_result_class = move || {
        let result_text = result.get();
        if result_text.is_empty() {
            return "".to_string();
        }
        let base_classes = "p-6 rounded-xl text-center font-bold text-lg border-2 transition-colors";
        if result_text.contains("Good guess") {
            format!(
                "{} bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-800 text-green-800 dark:text-green-300",
                base_classes
            )
        } else if result_text.contains("Bad") {
            format!("{} bg-red-50 dark:bg-red-900/30 border-red-200 dark:border-red-800 text-red-800 dark:text-red-300", base_classes)
        } else {
            format!("{} bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-300", base_classes)
        }
    };

    view! {
        <div class="max-w-6xl mx-auto my-8 p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors">
            // Game Header
            <div class="flex justify-between items-center mb-6 flex-wrap gap-4">
                <h2 class="text-2xl font-bold text-gray-800 dark:text-gray-100">"Kanji Game"</h2>
                <div class="flex items-center gap-4 flex-wrap">
                    <div class="bg-blue-50 dark:bg-blue-900/30 px-3 py-1 rounded-full text-sm text-blue-700 dark:text-blue-300 flex items-center whitespace-nowrap">
                        "Player: "
                        <span class="font-semibold ml-1">{move || player_name.get()}</span>
                    </div>
                    <div class="text-xl font-bold text-blue-500 dark:text-blue-400">
                        "Score: " {move || score.get()}
                    </div>
                    <button
                        on:click=handle_exit_game
                        class="bg-transparent hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 border border-gray-400 dark:border-gray-500 font-medium py-2 px-4 rounded transition-all hover:-translate-y-0.5 active:translate-y-0.5"
                    >
                        "Exit Game"
                    </button>
                </div>
            </div>

            // Lobby Info
            <div class="flex items-center gap-2 mb-6 p-2 bg-gray-100 dark:bg-gray-700 rounded text-sm relative transition-colors">
                <span class="text-gray-700 dark:text-gray-300">"Lobby ID:"</span>
                <span class="font-bold tracking-wider text-blue-600 dark:text-blue-400">{lobby_id.clone()}</span>
                <button
                    on:click=copy_lobby_id
                    class="ml-2 px-1 py-0.5 text-xs font-medium bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded transition-all duration-200 hover:bg-blue-50 dark:hover:bg-gray-500 hover:border-blue-400 hover:shadow-sm active:scale-95 active:bg-blue-100 dark:text-gray-200"
                    title="Copy Lobby ID"
                >
                    "Copy"
                </button>

                // Floating "Copied!" text using Show
                <Show when=move || is_copied.get()>
                    <div class="absolute -top-8 left-1/2 transform -translate-x-1/2 px-2 py-1 bg-green-500 text-white text-xs rounded shadow-lg animate-fade-in pointer-events-none">
                        "Copied!"
                    </div>
                </Show>
            </div>

            // Game Layout with Sidebar
            <div class="flex gap-8 flex-col lg:flex-row">
                // Main Game Area
                <div class="flex-1 space-y-8">
                    // Big Kanji Display Box
                    <div
                        class="flex justify-center items-center bg-gray-100 dark:bg-gray-700 rounded-lg border-2 border-gray-300 dark:border-gray-600 transition-colors"
                        style="height: 320px;"
                    >
                        <Show
                            when=move || is_loading.get()
                            fallback=move || {
                                view! {
                                    <div class="text-9xl leading-none text-gray-800 dark:text-gray-100 kanji-font select-none">
                                        {move || kanji.get()}
                                    </div>
                                }
                            }
                        >
                            <div class="text-lg text-gray-500 dark:text-gray-400">"Loading..."</div>
                        </Show>
                    </div>

                    // Input Area
                    <div class="space-y-4">
                        <input
                            node_ref=input_ref
                            type="text"
                            value=move || word.get()
                            on:input=handle_input
                            on:keydown=handle_key_press
                            placeholder="Enter a Japanese word with this kanji"
                            disabled=move || is_loading.get()
                            class="w-full p-3 text-lg border-2 border-gray-300 dark:border-gray-600 dark:bg-gray-900 dark:text-white rounded focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
                        />

                        <button
                            on:click=submit_word
                            disabled=move || {
                                is_loading.get() || word.get().trim().is_empty()
                                    || kanji.get().is_empty()
                            }
                            class="w-full bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 dark:disabled:bg-gray-600 disabled:cursor-not-allowed text-white font-semibold py-3 px-5 rounded transition-all duration-200 hover:-translate-y-0.5 active:translate-y-0.5 disabled:transform-none"
                        >
                            "Submit"
                        </button>
                    </div>

                    // Result Message
                    <Show when=move || !result.get().is_empty()>
                        <div class=get_result_class>{move || result.get()}</div>
                    </Show>

                    // Error Message
                    <Show when=move || !error_message.get().is_empty()>
                        <div class="p-4 rounded bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 text-center font-medium">
                            {move || error_message.get()}
                        </div>
                    </Show>

                    // Game Instructions
                    <div class="mt-8 pt-6 border-t border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm">
                        <p class="mb-2">"Type a Japanese word containing the displayed kanji."</p>
                        <p>"Click \"Submit\" to check your answer."</p>
                    </div>
                </div>

                // Scores Sidebar
                <div class="w-full lg:w-64 flex-shrink-0">
                    <Show when=move || !all_players.get().is_empty()>
                        <CompactPlayerScoresComponent
                            players=all_players.get()
                            current_player_id=player_id_signal
                            typing_status=typing_status.read_only()
                        />
                    </Show>
                </div>
            </div>
        </div>
    }
}
