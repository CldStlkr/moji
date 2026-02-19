use crate::api::update_lobby_settings;
use leptos::prelude::*;
use shared::{GameSettings, PlayerId, UpdateSettingsRequest};
use wasm_bindgen_futures::spawn_local;

/// Hook to handle updating lobby settings
pub fn use_lobby_settings(
    lobby_id: String,
    player_id: PlayerId,
) -> Callback<GameSettings> {
    Callback::new(move |new_settings: GameSettings| {
        let l_id = lobby_id.clone();
        let p_id = player_id.clone();
        spawn_local(async move {
            let req = UpdateSettingsRequest {
                player_id: p_id,
                settings: new_settings,
            };
            let _ = update_lobby_settings(&l_id, req).await;
        });
    })
}

/// Component for the Game Settings panel
#[component]
pub fn LobbySettingsPanel(
    settings: GameSettings,
    is_leader: bool,
    on_update: Callback<GameSettings>,
) -> impl IntoView
{
    // Handler for toggling difficulty
    let toggle_difficulty = {
        let settings = settings.clone();
        move |level: String| {
            if !is_leader {
                return;
            }
            let mut new_settings = settings.clone();
            if new_settings.difficulty_levels.contains(&level) {
                // Don't allow removing the last level
                if new_settings.difficulty_levels.len() > 1 {
                    new_settings.difficulty_levels.retain(|l| l != &level);
                    on_update.run(new_settings);
                }
            } else {
                new_settings.difficulty_levels.push(level);
                on_update.run(new_settings);
            }
        }
    };

    let toggle_weighted = {
        let settings = settings.clone();
        move |_| {
            if !is_leader {
                return;
            }
            let mut new_settings = settings.clone();
            new_settings.weighted = !new_settings.weighted;
            on_update.run(new_settings);
        }
    };

    view! {
        <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded border border-gray-200 dark:border-gray-600 transition-colors">
            <h4 class="font-semibold text-gray-700 dark:text-gray-200 mb-3">"Game Settings"</h4>

            // Difficulty Toggles
            <div class="mb-4">
                <span class="text-sm text-gray-600 dark:text-gray-400 block mb-2">"JLPT Levels:"</span>
                <div class="flex gap-2 flex-wrap">
                    {["N1", "N2", "N3", "N4", "N5"].into_iter().map(|level| {
                        let is_active = settings.difficulty_levels.contains(&level.to_string());
                        let level_str = level.to_string();
                        let interactable = is_leader;
                        let toggle = toggle_difficulty.clone();
                        view! {
                           <button
                               on:click=move |_| toggle(level_str.clone())
                               disabled=!interactable
                               class=format!(
                                   "px-3 py-1 rounded text-sm font-medium transition-colors border {}",
                                   if is_active {
                                       "bg-blue-500 dark:bg-blue-600 text-white border-blue-600 dark:border-blue-700"
                                   } else {
                                       "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
                                   }
                               )
                            >
                               {level}
                           </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Weighted Toggle
            <div class="flex items-center justify-between">
                <span class="text-sm text-gray-600 dark:text-gray-300">"Weighted Random (Prioritize more common kanji)"</span>
                <button
                    on:click=toggle_weighted
                    disabled=!is_leader
                    class=format!(
                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                        if settings.weighted { "bg-blue-600" } else { "bg-gray-200 dark:bg-gray-600" }
                    )
                >
                    <span
                        class=format!(
                            "inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200 ease-in-out {}",
                            if settings.weighted { "translate-x-6" } else { "translate-x-1" }
                        )
                    />
                </button>
            </div>
        </div>
    }
}
