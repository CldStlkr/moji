// lobby/mod.rs - Main lobby component
use leptos::{ev, prelude::*};
use crate::{api::join_lobby, error::{log_error, get_user_friendly_message}, persistence::SessionData};
use wasm_bindgen_futures::spawn_local;
use shared::{JoinLobbyRequest, LobbyInfo, PlayerId};

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
        set_current_lobby_id.set(lobby_id.clone());
        set_current_player_id.set(player_id.clone());
        set_in_lobby.set(true);

        on_lobby_joined(lobby_id, player_id);
    };

    let leave_lobby = move |_| {
        set_in_lobby.set(false);
        set_lobby_info.set(None);
        set_status.set("Left the lobby".to_string());
    };

    view! {
      <div class="my-8 mx-auto max-w-2xl">
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

#[component]
pub fn LobbyOrJoinComponent<F1, F2, F3>(
    lobby_id: ReadSignal<String>,
    player_id: ReadSignal<PlayerId>,
    player_name: ReadSignal<String>,
    on_lobby_joined: F1,
    on_game_started: F2,
    on_exit_lobby: F3,
) -> impl IntoView 
where 
    F1: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
    F3: Fn() + 'static + Copy + Send + Sync,
{
    let has_valid_session = move || {
        !player_id.get().is_empty() && !player_name.get().is_empty()
    };


    view! {
      <Show
        when=has_valid_session
        fallback=move || {
          view! {
            <LobbyJoinFormComponent target_lobby_id=lobby_id on_lobby_joined=on_lobby_joined />
          }
        }
      >
        <LobbyManagementWithGameTransition
          lobby_id=lobby_id
          player_id=player_id
          on_game_started=on_game_started
          on_exit_lobby=on_exit_lobby
        />
      </Show>
    }
}

#[component]
fn LobbyJoinFormComponent<F>(
    target_lobby_id: ReadSignal<String>,
    on_lobby_joined: F,
) -> impl IntoView 
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let player_name = RwSignal::new(String::new());
    
    let join_this_lobby = move |_: ev::MouseEvent| {
        let lobby_id = target_lobby_id.get();
        let name = player_name.get();
        
        if name.trim().is_empty() {
            set_status.set("Please enter your name".to_string());
            return;
        }
        
        spawn_local(async move {
            set_is_loading.set(true);
            set_status.set(format!("Joining lobby {}...", lobby_id));
            
            let request = JoinLobbyRequest {
                player_name: name.clone(),
            };
            
            match join_lobby(&lobby_id, request).await {
                Ok(response) => {
                    let player_id = PlayerId::from(
                        response
                            .get("player_id")
                            .and_then(|id| id.as_str())
                            .unwrap_or(""),
                    );
                    
                    if player_id.0.is_empty() {
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        let session = SessionData {
                            lobby_id: lobby_id.clone(),
                            player_id: player_id.clone(),
                            player_name: name.clone(),
                            is_in_game: false,
                        };
                        crate::persistence::save_session(&session);
                        on_lobby_joined(lobby_id, player_id);
                    }
                }
                Err(e) => {
                    log_error("Failed to join lobby", &e);
                    set_status.set(get_user_friendly_message(&e));
                }
            }
            set_is_loading.set(false);
        });
    };
    
    view! {
      <div class="p-8 my-8 mx-auto max-w-2xl bg-white rounded-lg shadow-lg">
        <h2 class="mb-6 text-2xl font-bold text-center text-gray-800">
          "Join Lobby: "
          <span class="font-mono font-bold tracking-wider text-blue-600">
            {move || target_lobby_id.get()}
          </span>
        </h2>

        <div class="space-y-4">
          <div class="space-y-2">
            <label for="player-name" class="block text-lg font-semibold text-gray-800">
              "Your Name:"
            </label>
            <input
              type="text"
              id="player-name"
              value=move || player_name.get()
              on:input=move |ev| player_name.set(event_target_value(&ev))
              placeholder="Enter your name"
              disabled=move || is_loading.get()
              class="p-3 w-full text-lg rounded border-2 border-gray-300 transition-colors focus:border-blue-500 focus:outline-none disabled:opacity-60"
            />
          </div>

          <button
            on:click=join_this_lobby
            disabled=move || is_loading.get() || player_name.get().trim().is_empty()
            class="py-3 px-6 w-full font-semibold text-white bg-blue-500 rounded transition-colors hover:bg-blue-600 disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            "Join This Lobby"
          </button>
        </div>

        <StatusMessage status=status />
      </div>
    }
}

#[component]
fn LobbyManagementWithGameTransition<F1, F2>(
    lobby_id: ReadSignal<String>,
    player_id: ReadSignal<PlayerId>,
    on_game_started: F1,
    on_exit_lobby: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (lobby_info, set_lobby_info) = signal::<Option<LobbyInfo>>(None);
    let (in_lobby, set_in_lobby) = signal(true);
    
    // Use lobby polling but with game transition handler
    use_lobby_polling(PollingConfig {
        in_lobby,
        current_lobby_id: lobby_id,
        current_player_id: player_id,
        set_lobby_info,
        set_status,
        set_in_lobby,
        on_lobby_joined: move |_, _| {
            // Game started - notify parent
            on_game_started();
        },
    });
    
    let leave_lobby = move |_| {
        set_in_lobby.set(false);
        on_exit_lobby();
    };
    
    view! {
      <LobbyManagementComponent
        lobby_info=lobby_info
        current_lobby_id=lobby_id
        current_player_id=player_id
        _is_loading=is_loading
        set_is_loading=set_is_loading
        _status=status
        set_status=set_status
        on_leave_lobby=leave_lobby
      />
    }
}
