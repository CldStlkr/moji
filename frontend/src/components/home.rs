use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::{
    components::lobby::LobbyJoinComponent,
    persistence::{load_session, clear_session},
};
use shared::{LobbyId, PlayerId};

#[component]
pub fn Home() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_path = RwSignal::new(None::<String>);

    let navigate_replace = navigate.clone();
    Effect::new(move |_| {
        if let Some(path) = navigate_path.get() {
            navigate_replace(&path, Default::default());
        }
    });

    // If a user navigates to the home page while holding a session, 
    // it implies they want to leave their current game, so we clear it.
    Effect::new(move |_| {
        if load_session().is_some() {
            clear_session();
        }
    });

    let (is_loading, set_is_loading) = signal(false);
    let (status, set_status) = signal(String::new());

    let handle_lobby_joined = move |new_lobby_id: LobbyId, _new_player_id: PlayerId| {
        navigate_path.set(Some(format!("/lobby/{}", new_lobby_id)));
    };

    view! {
        <div class="max-w-2xl mx-auto my-8 animate-page-entry">
            <LobbyJoinComponent
                is_loading=is_loading
                set_is_loading=set_is_loading
                status=status
                set_status=set_status
                on_lobby_joined=handle_lobby_joined
            />
        </div>
    }
}
