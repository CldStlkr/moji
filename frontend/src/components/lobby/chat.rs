use leptos::prelude::*;
use crate::{
    context::GameContext,
    styled_view,
};
use shared::ClientMessage;

styled_view!(chat_container, "flex flex-col w-full h-64 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden");
styled_view!(messages_box, "flex-1 min-h-0 w-full p-3 overflow-y-auto space-y-2 text-sm");
styled_view!(message_item, "flex flex-col");
styled_view!(sender_name, "font-bold text-blue-600 dark:text-blue-400 text-[10px] uppercase tracking-wider");
styled_view!(message_text, "text-gray-800 dark:text-gray-200 break-words");
styled_view!(input_container, "p-2 w-full shrink-0 border-t border-gray-200 dark:border-gray-700 flex gap-2");
styled_view!(chat_input, "flex-1 w-full min-w-0 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 dark:text-white");
styled_view!(send_btn, "shrink-0 bg-blue-500 hover:bg-blue-600 text-white rounded px-3 py-1.5 text-sm font-semibold transition-colors");

#[component]
pub fn ChatComponent() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let chat_messages = game_context.chat_messages;
    let send_message = game_context.send_message;
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let input = input_ref.get().expect("input exists");
        let msg = input.value();
        if !msg.trim().is_empty() {
            send_message.run(ClientMessage::Chat { message: msg });
            input.set_value("");
            let _ = input.focus();
        }
    };

    // Auto-scroll to bottom when new messages arrive
    let scroll_ref = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        chat_messages.get(); // Track changes
        if let Some(div) = scroll_ref.get() {
            div.set_scroll_top(div.scroll_height());
        }
    });

    view! {
        <div class=chat_container()>
            <div class=messages_box() node_ref=scroll_ref>
                <For
                    each=move || chat_messages.get()
                    key=|m| format!("{}-{}-{}", m.player_id, m.player_name, m.message) // Simple key
                    children=move |m| {
                        view! {
                            <div class=message_item()>
                                <span class=sender_name()>{m.player_name}</span>
                                <span class=message_text()>{m.message}</span>
                            </div>
                        }
                    }
                />
                <Show when=move || chat_messages.get().is_empty()>
                    <div class="text-center text-gray-400 italic py-8">"No messages yet..."</div>
                </Show>
            </div>
            <form on:submit=on_submit class=input_container()>
                <input
                    type="text"
                    node_ref=input_ref
                    placeholder="Type a message..."
                    class=chat_input()
                />
                <button type="submit" class=send_btn()>"Send"</button>
            </form>
        </div>
    }
}
