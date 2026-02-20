// lobby/mod.rs - Main lobby component
use leptos::prelude::*;
use shared::{LobbyInfo, PlayerId};
use wasm_bindgen_futures::spawn_local;

mod lobby_join;
mod lobby_management;
mod lobby_socket;
pub mod settings;

use lobby_join::LobbyJoinComponent;
use lobby_management::LobbyManagementComponent;
use lobby_socket::{use_lobby_socket, LobbySocketConfig};

// Re-export shared components
pub use lobby_management::{GameInstructions, StatusMessage};

#[component]
pub fn LobbyComponent<F>(
    on_lobby_joined: F,
    initial_lobby_id: Option<String>,
    initial_player_id: Option<PlayerId>,
    #[prop(optional)] on_left: Option<Callback<()>>,
) -> impl IntoView
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    // Lobby state - Initialize from props if available
    let has_initial = initial_lobby_id.is_some() && initial_player_id.is_some();
    let (in_lobby, set_in_lobby) = signal(has_initial);
    let (lobby_info, set_lobby_info) = signal::<Option<LobbyInfo>>(None);
    let (current_lobby_id, set_current_lobby_id) = signal(initial_lobby_id.unwrap_or_default());
    let (current_player_id, set_current_player_id) = signal(initial_player_id.unwrap_or_default());

    let navigate = leptos_router::hooks::use_navigate();
    let navigate_path = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if let Some(path) = navigate_path.get() {
            navigate(&path, Default::default());
        }
    });

    // Start polling when in lobby
    use_lobby_socket(LobbySocketConfig {
        in_lobby,
        current_lobby_id,
        current_player_id,
        set_lobby_info,
        on_lobby_joined,
    });

    let handle_lobby_joined = move |lobby_id: String, player_id: PlayerId| {
        let lobby_id_clone = lobby_id.clone();
        set_current_lobby_id.set(lobby_id);
        set_current_player_id.set(player_id);
        set_in_lobby.set(true);
        
        navigate_path.set(Some(format!("/join/{}", lobby_id_clone)));
        
        // NOTE: We do NOT call on_lobby_joined here.
        // on_lobby_joined (which sets is_in_game=true in Home) is only called
        // by lobby_socket.rs when GameState { Playing } arrives — i.e. when the
        // game actually starts, not when a player creates/joins the lobby.
    };

    let leave_lobby = move |_| {
        let lobby_id = current_lobby_id.get_untracked();
        let player_id = current_player_id.get_untracked();
        
        spawn_local(async move {
            let _ = crate::api::leave_lobby(&lobby_id, &player_id).await;
            set_in_lobby.set(false);
            set_lobby_info.set(None);
            set_status.set("Left the lobby".to_string());
            if let Some(cb) = on_left {
                cb.run(());
            }
        });
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
