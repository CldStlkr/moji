use leptos::prelude::*;
use crate::styled_view;

styled_view!(settings_grid_container, "grid grid-cols-1 gap-4");
styled_view!(settings_item_container, "flex flex-col gap-1.5");
styled_view!(settings_label, "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider");

#[component]
pub fn SettingsGrid(children: Children) -> impl IntoView {
    view! {
        <div class=settings_grid_container()>
            {children()}
        </div>
    }
}

#[component]
pub fn SettingsItem(
    label: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=settings_item_container()>
            <label class=settings_label()>{label}</label>
            <div class="mt-0.5">
                {children()}
            </div>
        </div>
    }
}
