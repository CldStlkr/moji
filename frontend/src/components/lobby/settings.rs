use crate::styled_view;
use leptos::prelude::*;
use shared::{GameSettings, UpdateSettingsRequest, update_lobby_settings};
use crate::context::GameContext;
use super::{ModeToggle, SettingsGrid, SettingsItem};

styled_view!(settings_container, "p-4 bg-gray-50 dark:bg-gray-700/50 rounded border border-gray-200 dark:border-gray-600 transition-colors space-y-6");
styled_view!(settings_title, "font-semibold text-gray-700 dark:text-gray-200");
styled_view!(label_text, "text-sm text-gray-600 dark:text-gray-400 block mb-2");
styled_view!(input_field, "w-full p-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white text-sm focus:ring-2 focus:ring-blue-500 focus:outline-none transition-all");
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
    set_is_loading: WriteSignal<bool>,
    set_status: WriteSignal<String>,
) -> Callback<GameSettings> {
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let lobby_id = game_context.lobby_id;
    let player_id = game_context.player_id;

    let run_api_action = crate::hooks::use_api_action(set_is_loading, set_status);
    
    Callback::new(move |new_settings: GameSettings| {
        let l_id = lobby_id.get();
        let p_id = player_id.get();
        
        run_api_action(Box::pin({
            let req = UpdateSettingsRequest {
                player_id: p_id.clone(),
                settings: new_settings.clone(),
            };
            async move {
                let _ = update_lobby_settings(l_id, req).await?;
                Ok(())
            }
        }));
    })
}

/// Component for the Game Settings panel
#[component]
pub fn LobbySettingsPanel(
    settings: Signal<GameSettings>,
    on_update: Callback<GameSettings>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    let is_leader = game_context.is_leader;
    // Handler for toggling difficulty
    let toggle_difficulty = {
        move |level: String| {
            if !is_leader.get() { return; }
            let mut new_settings = settings.get();
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
            if !is_leader.get() { return; }
            let mut new_settings = settings.get();
            new_settings.weighted = !new_settings.weighted;
            on_update.run(new_settings);
        }
    };

    view! {
        <div class=settings_container()>
            <h4 class=settings_title()>"Game Settings"</h4>

            <SettingsGrid>
                <SettingsItem label="Game Mode">
                    <ModeToggle 
                        selected=Signal::derive(move || settings.get().mode)
                        options=vec![
                            (shared::GameMode::Deathmatch, "Deathmatch"),
                            (shared::GameMode::Duel, "Duel"),
                            (shared::GameMode::Zen, "Zen"),
                        ]
                        on_change=Callback::new(move |mode| {
                            let mut new_settings = settings.get();
                            new_settings.mode = mode;
                            on_update.run(new_settings);
                        })
                    />
                </SettingsItem>

                <SettingsItem label="Content Type">
                    <ModeToggle 
                        selected=Signal::derive(move || settings.get().content_mode)
                        options=vec![
                            (shared::ContentMode::Kanji, "Kanji"),
                            (shared::ContentMode::Vocab, "Vocab"),
                        ]
                        on_change=Callback::new(move |content| {
                            let mut new_settings = settings.get();
                            new_settings.content_mode = content;
                            on_update.run(new_settings);
                        })
                    />
                </SettingsItem>

                // --- Mode Specific Settings ---
                <Show when=move || settings.get().mode == shared::GameMode::Deathmatch>
                    <SettingsItem label="Target Score">
                        <input
                            type="number"
                            min="1"
                            max="999"
                            value=move || settings.get().target_score.unwrap_or(10)
                            on:input={
                                move |ev| {
                                     if !is_leader.get() { return; }
                                     let val = event_target_value(&ev).parse::<u32>().ok();
                                     let mut new_settings = settings.get();
                                     new_settings.target_score = val;
                                     on_update.run(new_settings);
                                }
                            }
                            disabled=move || !is_leader.get()
                            class=input_field()
                        />
                         <p class="text-xs text-gray-400 mt-1">"First player to reach this score wins."</p>
                    </SettingsItem>
                </Show>

                <Show when=move || settings.get().mode == shared::GameMode::Duel>
                    <SettingsGrid>
                        <SettingsItem label="Initial Lives">
                            <input
                                type="number"
                                min="1"
                                max="99"
                                value=move || settings.get().initial_lives.unwrap_or(3)
                                on:input={
                                    move |ev| {
                                         if !is_leader.get() { return; }
                                         let val = event_target_value(&ev).parse::<u32>().ok();
                                         let mut new_settings = settings.get();
                                         new_settings.initial_lives = val;
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=move || !is_leader.get()
                                class=input_field()
                            />
                        </SettingsItem>

                         <SettingsItem label="Rules">
                             <div class="flex items-center justify-between p-2 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600">
                                <span class="text-xs text-gray-600 dark:text-gray-300">
                                    {move || if settings.get().content_mode == shared::ContentMode::Vocab { "Reuse Word on Miss" } else { "Reuse Kanji on Miss" }}
                                </span>
                                <button
                                    on:click={
                                        move |_| {
                                             if !is_leader.get() { return; }
                                             let mut new_settings = settings.get();
                                             new_settings.duel_allow_kanji_reuse = !new_settings.duel_allow_kanji_reuse;
                                             on_update.run(new_settings);
                                        }
                                    }
                                    disabled=move || !is_leader.get()
                                    class=move || toggle_switch(settings.get().duel_allow_kanji_reuse)
                                >
                                    <span class=move || toggle_knob(settings.get().duel_allow_kanji_reuse) />
                                </button>
                            </div>
                        </SettingsItem>
                    </SettingsGrid>
                </Show>

                // --- General Settings ---
                <SettingsItem label="JLPT Levels">
                    <div class="flex gap-2 flex-wrap">
                        {let diff_levels = move || settings.get().difficulty_levels;
                         ["N1", "N2", "N3", "N4", "N5"].into_iter().map(move |level| {
                            let level_str = level.to_string();
                            let toggle_difficulty = toggle_difficulty;
                            view! {
                               <button
                                   on:click=move |_| toggle_difficulty(level_str.clone())
                                   disabled=move || !is_leader.get()
                                   class=move || difficulty_btn(diff_levels().contains(&level.to_string()))
                                >
                                    {level}
                               </button>
                            }
                        }).collect_view()}
                    </div>
                </SettingsItem>

                <SettingsItem label="Randomization">
                    <div class="flex items-center justify-between p-2 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600">
                        <span class="text-xs text-gray-600 dark:text-gray-300">"Weighted (Common first)"</span>
                        <button
                            on:click=toggle_weighted
                            disabled=move || !is_leader.get()
                            class=move || toggle_switch(settings.get().weighted)
                        >
                            <span class=move || toggle_knob(settings.get().weighted) />
                        </button>
                    </div>
                </SettingsItem>

                <SettingsItem label="Visibility">
                    <div class="flex items-center justify-between p-2 bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600">
                        <div class="flex flex-col">
                            <span class="text-xs text-gray-600 dark:text-gray-300 font-medium">"Public Lobby"</span>
                            <span class="text-[10px] text-gray-400">"Show on home page"</span>
                        </div>
                        <button
                            on:click=move |_| {
                                if !is_leader.get() { return; }
                                let mut new_settings = settings.get();
                                new_settings.is_public = !new_settings.is_public;
                                on_update.run(new_settings);
                            }
                            disabled=move || !is_leader.get()
                            class=move || toggle_switch(settings.get().is_public)
                        >
                            <span class=move || toggle_knob(settings.get().is_public) />
                        </button>
                    </div>
                </SettingsItem>

                <SettingsItem label="Timing">
                    <div class="space-y-2">
                        <div class="flex items-center gap-2">
                            <input
                                type="number"
                                min="0"
                                max="300"
                                value=move || settings.get().time_limit_seconds.unwrap_or(0)
                                on:input={
                                    move |ev| {
                                         if !is_leader.get() { return; }
                                         let val = event_target_value(&ev).parse::<u32>().ok();
                                         let mut new_settings = settings.get();
                                         new_settings.time_limit_seconds = val.and_then(|v| if v == 0 { None } else { Some(v) });
                                         on_update.run(new_settings);
                                    }
                                }
                                disabled=move || !is_leader.get()
                                class=input_field()
                            />
                            <span class="text-sm text-gray-500">"sec"</span>
                        </div>
                        <p class="text-xs text-gray-400">"Set to 0 for no limit."</p>
                    </div>
                </SettingsItem>
            </SettingsGrid>
        </div>
    }
}
