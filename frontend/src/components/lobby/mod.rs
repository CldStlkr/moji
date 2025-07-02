// lobby/mod.rs - Main lobby component
use leptos::prelude::*;
use shared::{LobbyInfo, PlayerId};

mod lobby_join;
mod lobby_management;
mod lobby_polling;

use lobby_join::LobbyJoinComponent;
use lobby_management::LobbyManagementComponent;
use lobby_polling::{use_lobby_polling, PollingConfig};

// Re-export shared components
pub use lobby_management::{GameInstructions, StatusMessage};

#[component]
pub fn LobbyComponent<F>(on_lobby_joined: F) -> impl IntoView
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    // Lobby state
    let (in_lobby, set_in_lobby) = signal(false);
    let (lobby_info, set_lobby_info) = signal::<Option<LobbyInfo>>(None);
    let (current_lobby_id, set_current_lobby_id) = signal(String::new());
    let (current_player_id, set_current_player_id) = signal(PlayerId::default());

    // Start polling when in lobby
    use_lobby_polling(PollingConfig {
        in_lobby,
        current_lobby_id,
        current_player_id,
        set_lobby_info,
        set_status,
        set_in_lobby,
        on_lobby_joined,
    });

    let handle_lobby_joined = move |lobby_id: String, player_id: PlayerId| {
        set_current_lobby_id.set(lobby_id);
        set_current_player_id.set(player_id);
        set_in_lobby.set(true);
    };

    let leave_lobby = move |_| {
        set_in_lobby.set(false);
        set_lobby_info.set(None);
        set_status.set("Left the lobby".to_string());
    };

    view! {
        <div class="max-w-2xl mx-auto my-8">
            <Show
                when=move || !in_lobby.get()
                fallback=move || {
                    view! {
                        <LobbyManagementComponent
                            lobby_info=lobby_info
                            current_lobby_id=current_lobby_id
                            current_player_id=current_player_id
                            _is_loading=is_loading
                            set_is_loading=set_is_loading
                            _status=status
                            set_status=set_status
                            on_leave_lobby=leave_lobby
                        />
                    }
                }
            >
                <LobbyJoinComponent
                    is_loading=is_loading
                    set_is_loading=set_is_loading
                    status=status
                    set_status=set_status
                    on_lobby_joined=handle_lobby_joined
                />
            </Show>
        </div>
    }
}
