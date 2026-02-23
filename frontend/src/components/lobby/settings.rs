
use leptos::prelude::*;
use shared::{GameSettings, LobbyId, PlayerId, UpdateSettingsRequest, GameMode, update_lobby_settings};
use wasm_bindgen_futures::spawn_local;

/// Hook to handle updating lobby settings
pub fn use_lobby_settings(
    lobby_id: LobbyId,
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
            let _ = update_lobby_settings(l_id, req).await;
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
    let settings = StoredValue::new(settings);
    // Handler for toggling difficulty
    let toggle_difficulty = {
        move |level: String| {
            if !is_leader { return; }
            let mut new_settings = settings.get_value();
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
        move |_| {
            if !is_leader { return; }
            let mut new_settings = settings.get_value();
            new_settings.weighted = !new_settings.weighted;
            on_update.run(new_settings);
        }
    };




    view! {
        <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded border border-gray-200 dark:border-gray-600 transition-colors space-y-6">
            <h4 class="font-semibold text-gray-700 dark:text-gray-200">"Game Settings"</h4>

            // --- Game Mode Selection ---
            <div>
                <span class="text-sm text-gray-600 dark:text-gray-400 block mb-2">"Game Mode:"</span>
                <div class="flex gap-2">
                    <button
                        on:click={
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.mode = GameMode::Deathmatch;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || format!(
                            "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border {}",
                            if settings.get_value().mode == GameMode::Deathmatch {
                                "bg-indigo-600 text-white border-indigo-600"
                            } else {
                                "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
                            }
                        )
                    >
                        "Deathmatch (Race)"
                    </button>
                    <button
                        on:click={
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.mode = GameMode::Duel;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                         class=move || format!(
                            "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border {}",
                            if settings.get_value().mode == GameMode::Duel {
                                "bg-indigo-600 text-white border-indigo-600"
                            } else {
                                "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
                            }
                        )
                    >
                        "Duel (Survival)"
                    </button>
                </div>
            </div>

            // --- Content Mode Selection ---
            <div>
                <span class="text-sm text-gray-600 dark:text-gray-400 block mb-2">"Content Mode:"</span>
                <div class="flex gap-2">
                    <button
                        on:click={
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.content_mode = shared::ContentMode::Kanji;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || format!(
                            "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border {}",
                            if settings.get_value().content_mode == shared::ContentMode::Kanji {
                                "bg-indigo-600 text-white border-indigo-600"
                            } else {
                                "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
                            }
                        )
                    >
                        "Kanji"
                    </button>
                    <button
                        on:click={
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.content_mode = shared::ContentMode::Vocab;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || format!(
                            "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border {}",
                            if settings.get_value().content_mode == shared::ContentMode::Vocab {
                                "bg-indigo-600 text-white border-indigo-600"
                            } else {
                                "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
                            }
                        )
                    >
                        "Vocab"
                    </button>
                </div>
            </div>

            // --- Mode Specific Settings ---
            <div class="p-3 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600">
                <Show when=move || settings.get_value().mode == GameMode::Deathmatch>
                    <div class="flex flex-col gap-2">
                        <label class="text-sm text-gray-600 dark:text-gray-400">"Target Score to Win:"</label>
                        <input
                            type="number"
                            min="1"
                            max="999"
                            value=move || settings.get_value().target_score.unwrap_or(10)
                            on:input={
                                move |ev| {
                                     if !is_leader { return; }
                                     let val = event_target_value(&ev).parse::<u32>().ok();
                                     let mut new_settings = settings.get_value();
                                     new_settings.target_score = val;
                                     on_update.run(new_settings);
                                }
                            }
                            disabled=!is_leader
                            class="p-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        />
                         <p class="text-xs text-gray-500">"First player to reach this score wins."</p>
                    </div>
                </Show>

                <Show when=move || settings.get_value().mode == GameMode::Duel>
                    <div class="space-y-4">
                        <div class="flex flex-col gap-2">
                            <label class="text-sm text-gray-600 dark:text-gray-400">"Initial Lives:"</label>
                            <input
                                type="number"
                                min="1"
                                max="99"
                                value=move || settings.get_value().initial_lives.unwrap_or(3)
                                on:input={
                                    move |ev| {
                                         if !is_leader { return; }
                                         let val = event_target_value(&ev).parse::<u32>().ok();
                                         let mut new_settings = settings.get_value();
                                         new_settings.initial_lives = val;
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=!is_leader
                                class="p-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            />
                        </div>

                         <div class="flex items-center justify-between">
                            <span class="text-sm text-gray-600 dark:text-gray-300">"Reuse Kanji on Miss"</span>
                            <button
                                on:click={
                                    move |_| {
                                         if !is_leader { return; }
                                         let mut new_settings = settings.get_value();
                                         new_settings.duel_allow_kanji_reuse = !new_settings.duel_allow_kanji_reuse;
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=!is_leader
                                class=move || format!(
                                    "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                                    if settings.get_value().duel_allow_kanji_reuse { "bg-blue-600" } else { "bg-gray-200 dark:bg-gray-600" }
                                )
                            >
                                <span
                                    class=move || format!(
                                        "inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200 ease-in-out {}",
                                        if settings.get_value().duel_allow_kanji_reuse { "translate-x-6" } else { "translate-x-1" }
                                    )
                                />
                            </button>
                        </div>
                        <p class="text-xs text-gray-500">"If enabled, next player faces the same kanji after a miss."</p>
                    </div>
                </Show>
            </div>


            // --- General Settings ---

            // Difficulty Toggles
            <div>
                <span class="text-sm text-gray-600 dark:text-gray-400 block mb-2">"JLPT Levels:"</span>
                <div class="flex gap-2 flex-wrap">
                    {["N1", "N2", "N3", "N4", "N5"].into_iter().map(|level| {
                        let is_active = settings.get_value().difficulty_levels.contains(&level.to_string());
                        let level_str = level.to_string();
                        let interactable = is_leader;
                        view! {
                           <button
                               on:click=move |_| toggle_difficulty(level_str.clone())
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
                    class=move || format!(
                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                        if settings.get_value().weighted { "bg-blue-600" } else { "bg-gray-200 dark:bg-gray-600" }
                    )
                >
                    <span
                        class=move || format!(
                            "inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200 ease-in-out {}",
                            if settings.get_value().weighted { "translate-x-6" } else { "translate-x-1" }
                        )
                    />
                </button>
            </div>
        </div>
    }
}
