
use crate::styled_view;
use leptos::prelude::*;
use shared::{GameSettings, LobbyId, PlayerId, UpdateSettingsRequest, GameMode, update_lobby_settings};
use wasm_bindgen_futures::spawn_local;

styled_view!(settings_container, "p-4 bg-gray-50 dark:bg-gray-700/50 rounded border border-gray-200 dark:border-gray-600 transition-colors space-y-6");
styled_view!(settings_title, "font-semibold text-gray-700 dark:text-gray-200");
styled_view!(label_text, "text-sm text-gray-600 dark:text-gray-400 block mb-2");
styled_view!(mode_btn, is_active: bool, 
    "flex-1 py-2 px-4 rounded text-sm font-medium transition-colors border", 
    if is_active { "bg-indigo-600 text-white border-indigo-600" } else { "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700" }
);
styled_view!(settings_group, "p-3 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600");
styled_view!(input_field, "p-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white");
styled_view!(toggle_switch, is_active: bool, 
    "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800", 
    if is_active { "bg-blue-600" } else { "bg-gray-200 dark:bg-gray-600" }
);
styled_view!(toggle_knob, is_active: bool, 
    "inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200 ease-in-out", 
    if is_active { "translate-x-6" } else { "translate-x-1" }
);
styled_view!(difficulty_btn, is_active: bool, 
    "px-3 py-1 rounded text-sm font-medium transition-colors border", 
    if is_active { "bg-blue-500 dark:bg-blue-600 text-white border-blue-600 dark:border-blue-700" } else { "bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700" }
);

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
        let on_update = on_update.clone();
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
        let on_update = on_update.clone();
        move |_| {
            if !is_leader { return; }
            let mut new_settings = settings.get_value();
            new_settings.weighted = !new_settings.weighted;
            on_update.run(new_settings);
        }
    };

    view! {
        <div class=settings_container()>
            <h4 class=settings_title()>"Game Settings"</h4>

            // --- Game Mode Selection ---
            <div>
                <span class=label_text()>"Game Mode:"</span>
                <div class="flex gap-2">
                    <button
                        on:click={
                            let on_update = on_update.clone();
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.mode = GameMode::Deathmatch;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || mode_btn(settings.get_value().mode == GameMode::Deathmatch)
                    >
                        "Deathmatch (Race)"
                    </button>
                    <button
                        on:click={
                            let on_update = on_update.clone();
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.mode = GameMode::Duel;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || mode_btn(settings.get_value().mode == GameMode::Duel)
                    >
                        "Duel (Survival)"
                    </button>
                </div>
            </div>

            // --- Content Mode Selection ---
            <div>
                <span class=label_text()>"Content Mode:"</span>
                <div class="flex gap-2">
                    <button
                        on:click={
                            let on_update = on_update.clone();
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.content_mode = shared::ContentMode::Kanji;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || mode_btn(settings.get_value().content_mode == shared::ContentMode::Kanji)
                    >
                        "Kanji"
                    </button>
                    <button
                        on:click={
                            let on_update = on_update.clone();
                            move |_| {
                                if !is_leader { return; }
                                let mut new_settings = settings.get_value();
                                new_settings.content_mode = shared::ContentMode::Vocab;
                                on_update.run(new_settings);
                            }
                        }
                        disabled=!is_leader
                        class=move || mode_btn(settings.get_value().content_mode == shared::ContentMode::Vocab)
                    >
                        "Vocab"
                    </button>
                </div>
            </div>

            // --- Mode Specific Settings ---
            <div class=settings_group()>
                <Show when=move || settings.get_value().mode == GameMode::Deathmatch>
                    <div class="flex flex-col gap-2">
                        <label class="text-sm text-gray-600 dark:text-gray-400">"Target Score to Win:"</label>
                        <input
                            type="number"
                            min="1"
                            max="999"
                            value=move || settings.get_value().target_score.unwrap_or(10)
                            on:input={
                                let on_update = on_update.clone();
                                move |ev| {
                                     if !is_leader { return; }
                                     let val = event_target_value(&ev).parse::<u32>().ok();
                                     let mut new_settings = settings.get_value();
                                     new_settings.target_score = val;
                                     on_update.run(new_settings);
                                }
                            }
                            disabled=!is_leader
                            class=input_field()
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
                                    let on_update = on_update.clone();
                                    move |ev| {
                                         if !is_leader { return; }
                                         let val = event_target_value(&ev).parse::<u32>().ok();
                                         let mut new_settings = settings.get_value();
                                         new_settings.initial_lives = val;
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=!is_leader
                                class=input_field()
                            />
                        </div>

                         <div class="flex items-center justify-between">
                            <span class="text-sm text-gray-600 dark:text-gray-300">"Reuse Kanji on Miss"</span>
                            <button
                                on:click={
                                    let on_update = on_update.clone();
                                    move |_| {
                                         if !is_leader { return; }
                                         let mut new_settings = settings.get_value();
                                         new_settings.duel_allow_kanji_reuse = !new_settings.duel_allow_kanji_reuse;
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=!is_leader
                                class=move || toggle_switch(settings.get_value().duel_allow_kanji_reuse)
                            >
                                <span class=move || toggle_knob(settings.get_value().duel_allow_kanji_reuse) />
                            </button>
                        </div>
                        <p class="text-xs text-gray-500">"If enabled, next player faces the same kanji after a miss."</p>
                    </div>
                </Show>
            </div>


            // --- General Settings ---

            // Difficulty Toggles
            <div>
                <span class=label_text()>"JLPT Levels:"</span>
                <div class="flex gap-2 flex-wrap">
                    {["N1", "N2", "N3", "N4", "N5"].into_iter().map(|level| {
                        let is_active = settings.get_value().difficulty_levels.contains(&level.to_string());
                        let level_str = level.to_string();
                        let toggle_difficulty = toggle_difficulty.clone();
                        view! {
                           <button
                               on:click=move |_| toggle_difficulty(level_str.clone())
                               disabled=!is_leader
                               class=difficulty_btn(is_active)
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
                    class=move || toggle_switch(settings.get_value().weighted)
                >
                    <span class=move || toggle_knob(settings.get_value().weighted) />
                </button>
            </div>
        </div>
    }
}
