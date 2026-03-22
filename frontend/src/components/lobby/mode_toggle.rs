use leptos::prelude::*;
use crate::styled_view;
use crate::context::GameContext;

styled_view!(mode_btn, is_active: bool,
    "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border",
    if is_active { "bg-indigo-600 text-white border-indigo-600" } else { "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700" }
);

#[component]
pub fn ModeToggle<T>(
    selected: Signal<T>,
    options: Vec<(T, &'static str)>,
    on_change: Callback<T>,
) -> impl IntoView 
where T: Clone + PartialEq + Send + Sync + 'static
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let is_leader = game_context.is_leader;

    view! {
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
            {options.into_iter().map(|(value, label)| {
                let v = value.clone();
                let is_active = Signal::derive(move || selected.get() == v.clone());
                let v2 = value.clone();
                view! {
                    <button
                        on:click=move |_| if is_leader.get() { on_change.run(v2.clone()) }
                        disabled=move || !is_leader.get()
                        class=move || mode_btn(is_active.get())
                    >
                        {label}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
