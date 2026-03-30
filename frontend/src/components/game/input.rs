use crate::styled_view;
use leptos::ev;
use leptos::prelude::*;
use leptos_dom::helpers::window_event_listener;

styled_view!(game_input_field, "w-full p-3 text-lg border-2 border-gray-300 dark:border-gray-600 dark:bg-gray-900 dark:text-white rounded focus:border-blue-500 dark:focus:border-blue-400 focus:outline-none transition-colors disabled:opacity-60 disabled:cursor-not-allowed");
styled_view!(game_submit_button, disabled: bool,
    "flex-1 text-white font-semibold py-3 px-5 rounded transition-all duration-200",
    if disabled {
        "bg-gray-400 dark:bg-gray-600 cursor-not-allowed transform-none"
    } else {
        "bg-blue-500 hover:bg-blue-600 hover:-translate-y-0.5 active:translate-y-0.5"
    }
);
styled_view!(game_skip_button, disabled: bool,
    "text-white font-semibold py-3 px-5 rounded transition-all duration-200",
    if disabled {
        "bg-gray-400 dark:bg-gray-600 cursor-not-allowed transform-none"
    } else {
        "bg-red-500 hover:bg-red-600 hover:-translate-y-0.5 active:translate-y-0.5"
    }
);

use crate::context::{GameContext, InGameContext};

#[component]
pub fn GameInput() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let in_game_context = use_context::<InGameContext>().expect("InGameContext missing");

    let lobby_info = game_context.lobby_info;
    let player_id = game_context.player_id;
    let prompt = game_context.prompt;
    let send_message = game_context.send_message;

    let word = in_game_context.word;
    let is_loading = in_game_context.is_loading;
    let input_ref = in_game_context.input_ref;
    let on_submit = in_game_context.on_submit;
    let on_skip = in_game_context.on_skip;

    let content_mode = Signal::derive(move || {
        lobby_info.get().map(|i| i.settings.content_mode).unwrap_or_default()
    });

    let is_spectator = Signal::derive(move || {
        lobby_info.get().map(|i| {
            i.players.iter().any(|p| p.id == player_id.get() && p.is_spectator)
        }).unwrap_or(false)
    });

    let disabled = Signal::derive(move || {
         if is_spectator.get() {
             return true;
         }
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
    });

    let handle = window_event_listener(ev::keydown, move |e: ev::KeyboardEvent| {
        if is_spectator.get() { return; }

        let key = e.key();
        if e.meta_key() || e.ctrl_key() || e.alt_key()
            || key == "Tab" || key == "Escape" || key.starts_with('F')
            { return; }

        // If the user is already focused on an input or textarea (like chat), don't steal focus
        if let Some(active) = document().active_element() {
            let tag = active.tag_name().to_uppercase();
            if tag == "INPUT" || tag == "TEXTAREA" {
                return;
            }
        }

        if let Some(input) = input_ref.get() {
            if !input.is_same_node(document()
                .active_element().as_ref()
                .map(|e| e.as_ref()))
            {
                e.prevent_default();
                let _ = input.focus();
            }
        }
    });

    on_cleanup(move || {
        handle.remove();
    });

    let is_btn_disabled = move || {
        is_loading.get() || word.get().trim().is_empty()
            || prompt.get().is_empty() || disabled.get()
    };

    let handle_input = move |ev| {
        let val = event_target_value(&ev);
        word.set(val.clone());
        send_message.run(shared::ClientMessage::Typing { input: val });
    };

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_btn_disabled() {
            on_submit.run(());
        }
    };

    view! {
        <div class="space-y-4">
            <input
                node_ref=input_ref
                type="text"
                value=move || word.get()
                on:input=handle_input
                on:keydown=handle_keydown

                placeholder=move || {
                    if is_spectator.get() {
                        "Spectating match in progress..."
                    } else if content_mode.get() == shared::ContentMode::Vocab {
                        "Enter the reading in hiragana"
                    } else {
                        "Enter a Japanese word with this kanji"
                    }
                }
                disabled=move || is_loading.get() || disabled.get()
                class=game_input_field()
            />

            <div class="flex gap-4 w-full">
                <button
                    on:click=move |_| on_submit.run(())
                    disabled=is_btn_disabled
                    class=move || game_submit_button(is_btn_disabled())
                >
                    "Submit"
                </button>
                <button
                    on:click=move |_| on_skip.run(())
                    disabled=move || is_loading.get() || disabled.get()
                    class=move || game_skip_button(is_loading.get() || disabled.get())
                >
                    "Skip"
                </button>
            </div>
        </div>
    }
}
