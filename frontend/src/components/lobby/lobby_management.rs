// Component for managing lobby state
use crate::{
    api::start_game,
    error::{get_user_friendly_message, log_error},
};
use leptos::ev;
use leptos::prelude::*;
use shared::{LobbyInfo, PlayerData, PlayerId, StartGameRequest};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LobbyManagementComponent<F>(
    lobby_info: ReadSignal<Option<LobbyInfo>>,
    current_lobby_id: ReadSignal<String>,
    current_player_id: ReadSignal<PlayerId>,
    _is_loading: ReadSignal<bool>,
    set_is_loading: WriteSignal<bool>,
    _status: ReadSignal<String>,
    set_status: WriteSignal<String>,
    on_leave_lobby: F,
) -> impl IntoView
where
    F: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    let is_copied = RwSignal::new(false);
    let start_game_action = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        let player_id = current_player_id.get();

        spawn_local(async move {
            set_is_loading.set(true);
            set_status.set("Starting game...".to_string());

            let request = StartGameRequest {
                player_id: player_id.clone(),
            };

            match start_game(&lobby_id, request).await {
                Ok(_) => {
                    set_status.set("Game starting...".to_string());
                }
                Err(e) => {
                    log_error("Failed to start game", &e);
                    set_status.set(get_user_friendly_message(&e));
                }
            }
            set_is_loading.set(false);
        });
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await {
                Ok(_) => {
                    is_copied.set(true);

                    gloo_timers::future::TimeoutFuture::new(1000).await;
                    is_copied.set(false);
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard")
                }
            }
        });
    };

    let is_leader = move || {
        if let Some(info) = lobby_info.get() {
            info.leader_id == current_player_id.get()
        } else {
            false
        }
    };

    view! {
      <div class="p-8 my-8 mx-auto max-w-2xl bg-white rounded-lg shadow-lg">
        <LobbyHeader
          lobby_id=current_lobby_id
          on_copy_id=copy_lobby_id
          is_copied=is_copied.read_only()
        />

        <Show
          when=move || lobby_info.get().is_some()
          fallback=|| {
            view! { <div class="text-lg text-center text-gray-500">"Loading lobby info..."</div> }
          }
        >
          {move || {
            lobby_info
              .get()
              .map(|info| {
                view! {
                  <LobbyDetails
                    lobby_info=info
                    current_player_id=current_player_id
                    is_leader=is_leader
                    on_start_game=start_game_action
                    on_leave_lobby=on_leave_lobby
                  />
                }
              })
          }}
        </Show>
      </div>
    }
}

#[component]
fn LobbyHeader<F>(
    lobby_id: ReadSignal<String>,
    on_copy_id: F,
    is_copied: ReadSignal<bool>,
) -> impl IntoView
where
    F: Fn(ev::MouseEvent) + 'static + Copy,
{
    view! {
      <div class="flex relative justify-between items-center mb-6">
        <h2 class="text-2xl font-bold text-gray-800">
          "Lobby: "
          <span class="font-mono font-bold tracking-wider text-blue-600">{lobby_id.get()}</span>
        </h2>
        <button
          on:click=on_copy_id
          class="py-1 px-2 text-xs font-medium bg-white rounded border border-gray-300 transition-colors hover:bg-gray-50"
          title="Copy Lobby ID"
        >
          "Copy"
        </button>
      </div>

      // Floating "Copied!" text using Show
      <Show when=move || is_copied.get()>
        <div class="absolute -top-8 left-1/2 py-1 px-2 text-xs text-white bg-green-500 rounded shadow-lg transform -translate-x-1/2 pointer-events-none animate-fade-in">
          "Copied!"
        </div>
      </Show>
    }
}

#[component]
fn LobbyDetails<F1, F2>(
    lobby_info: LobbyInfo,
    current_player_id: ReadSignal<PlayerId>,
    is_leader: impl Fn() -> bool + 'static + Copy + Send + Sync,
    on_start_game: F1,
    on_leave_lobby: F2,
) -> impl IntoView
where
    F1: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
    F2: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    let player_count = lobby_info.players.len();
    let max_players = lobby_info.settings.max_players;
    let leader_id = lobby_info.leader_id.clone();
    let status = lobby_info.status;

    view! {
      <div class="space-y-6">
        <PlayersList
          players=lobby_info.players
          current_player_id=current_player_id
          leader_id=leader_id
          player_count=player_count
          max_players=max_players
        />

        <div class="flex gap-2 items-center p-3 bg-gray-50 rounded">
          <span class="text-gray-700">"Status:"</span>
          <span class="font-medium text-blue-600">{format!("{:?}", status)}</span>
        </div>

        <LobbyActions
          is_leader=is_leader
          player_count=player_count
          on_start_game=on_start_game
          on_leave_lobby=on_leave_lobby
        />
      </div>
    }
}

#[component]
fn PlayersList(
    players: Vec<PlayerData>,
    current_player_id: ReadSignal<PlayerId>,
    leader_id: PlayerId,
    player_count: usize,
    max_players: u32,
) -> impl IntoView {
    view! {
      <div class="space-y-4">
        <h3 class="pb-2 text-xl font-semibold text-blue-600 border-b border-gray-200">
          "Players (" {player_count} "/" {max_players} ")"
        </h3>
        <ul class="space-y-2">
          {players
            .into_iter()
            .map(|player| {
              let is_current = player.id == current_player_id.get();
              let is_leader = player.id == leader_id;
              view! {
                <li class=format!(
                  "flex justify-between items-center p-3 rounded border-b border-gray-200 {}",
                  if is_current { "bg-blue-50 font-semibold" } else { "" },
                )>
                  <span class="font-medium">{player.name}</span>
                  <div class="flex gap-2 items-center">
                    <Show when=move || is_leader>
                      <span class="text-lg" title="Lobby Leader">
                        "👑"
                      </span>
                    </Show>
                    <Show when=move || is_current>
                      <span class="text-sm font-medium text-blue-600">"(You)"</span>
                    </Show>
                  </div>
                </li>
              }
            })
            .collect_view()}
        </ul>
      </div>
    }
}

#[component]
fn LobbyActions<F1, F2>(
    is_leader: impl Fn() -> bool + 'static + Copy + Send + Sync,
    player_count: usize,
    on_start_game: F1,
    on_leave_lobby: F2,
) -> impl IntoView
where
    F1: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
    F2: Fn(ev::MouseEvent) + 'static + Copy + Send + Sync,
{
    view! {
      <div class="flex flex-col gap-4 my-6">
        <Show
          when=is_leader
          fallback=|| {
            view! {
              <p class="py-4 italic text-center text-gray-600">
                "Waiting for leader to start the game..."
              </p>
            }
          }
        >
          <button
            on:click=on_start_game
            disabled=move || player_count < 1
            class="py-3 px-6 font-semibold text-white bg-green-500 rounded transition-colors hover:bg-green-600 disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            "Start Game"
          </button>
          <Show when=move || player_count < 1>
            <p class="font-medium text-center text-orange-600">
              "Need at least 2 players to start"
            </p>
          </Show>
        </Show>

        <button
          on:click=on_leave_lobby
          class="py-2 px-4 font-medium text-gray-600 bg-transparent rounded border border-gray-400 transition-colors hover:bg-gray-50"
        >
          "Leave Lobby"
        </button>
      </div>
    }
}

#[component]
pub fn StatusMessage(status: ReadSignal<String>) -> impl IntoView {
    view! {
      <Show when=move || !status.get().is_empty()>
        <div class=move || {
          let base_classes = "my-4 p-3 rounded text-center font-medium";
          if status.get().contains("Error") {
            format!("{} bg-red-100 text-red-700", base_classes)
          } else {
            format!("{} bg-gray-100 text-gray-700", base_classes)
          }
        }>{move || status.get()}</div>
      </Show>
    }
}

#[component]
pub fn GameInstructions() -> impl IntoView {
    view! {
      <div class="pt-4 mt-8 border-t border-gray-200">
        <h3 class="mb-3 text-lg font-semibold text-gray-600">"How to Play"</h3>
        <div class="space-y-2 text-gray-700">
          <p>"Create a new game or join an existing one with a lobby ID."</p>
          <p>"Once in a game, you'll be shown a kanji character."</p>
          <p>"Type a Japanese word that contains that kanji and submit it to score points!"</p>
        </div>
      </div>
    }
}
