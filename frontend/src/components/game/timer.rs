use leptos::prelude::*;
use crate::context::GameContext;
use std::time::Duration;

#[component]
pub fn TimerBar() -> impl IntoView {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let lobby_info = game_context.lobby_info;
    let progress = RwSignal::new(100.0);
    let expires_at = game_context.expires_at;
    let time_limit = Memo::new(move |_| {
        lobby_info.get().and_then(|info| info.settings.time_limit_seconds)
    });
    let is_active = Memo::new(move |_| time_limit.get().unwrap_or(0) > 0);

    Effect::new(move |_| {
        let current_expires = expires_at.get();
        let limit = time_limit.get();

        if current_expires.is_none() || limit.is_none() || limit.unwrap_or(0) == 0 {
            progress.set(100.0);
            return current_expires;
        }

        let limit_secs = limit.unwrap() as f64;
        let expires_ms = current_expires.unwrap() as f64;

        let handle = set_interval_with_handle(
            move || {
                let now = js_sys::Date::now();
                let remaining_ms = (expires_ms - now).max(0.0);
                let remaining_secs = remaining_ms / 1000.0;
                let pct = (remaining_secs / limit_secs) * 100.0;
                progress.set(pct);
            },
            Duration::from_millis(100),
        ).expect("failed to set interval");

        on_cleanup(move || {
            handle.clear();
        });

        current_expires
    });

    let bar_color = Signal::derive(move || {
        let p = progress.get();
        if p > 60.0 {
            "bg-green-500"
        } else if p > 30.0 {
            "bg-yellow-400"
        } else {
            "bg-red-500"
        }
    });

    view! {
        <Show when=move || is_active.get()>
            <div class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden mb-4">
                <div
                    class=move || format!("h-full transition-all duration-100 ease-linear {}", bar_color.get())
                    style:width=move || format!("{}%", progress.get())
                ></div>
            </div>
        </Show>
    }
}
