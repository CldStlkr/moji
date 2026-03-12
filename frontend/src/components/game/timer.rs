use leptos::prelude::*;
use std::time::Duration;

#[component]
pub fn TimerBar(
    #[prop(into)] time_limit: Signal<Option<u32>>,
    #[prop(into)] reset_trigger: Signal<String>,
) -> impl IntoView {
    let progress = RwSignal::new(100.0);
    let is_active = Signal::derive(move || time_limit.get().is_some() && time_limit.get().unwrap_or(0) > 0);

    Effect::new(move |_| {
        let trigger = reset_trigger.get();
        let limit = time_limit.get();
        if trigger.is_empty() || limit.is_none() || limit.unwrap_or(0) == 0 {
            progress.set(100.0);
            return;
        }

        let limit_secs = limit.unwrap() as f64;
        let start_time = js_sys::Date::now();
        progress.set(100.0);

        let handle = set_interval_with_handle(
            move || {
                let now = js_sys::Date::now();
                let elapsed = (now - start_time) / 1000.0;
                let remaining = (limit_secs - elapsed).max(0.0);
                let pct = (remaining / limit_secs) * 100.0;
                progress.set(pct);
            },
            Duration::from_millis(100),
        ).expect("failed to set interval");

        on_cleanup(move || {
            handle.clear();
        });
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
