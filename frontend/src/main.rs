use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    path,
};
use wasm_bindgen::prelude::*;

use moji_frontend::components;

use components::{
    auth_modal::AuthModal,
    user_menu::UserMenu,
    home::Home,
    lobby_page::LobbyPage,
};
use moji_frontend::context::{AuthContext, User};
use moji_frontend::persistence::load_auth;

#[component]
fn App() -> impl IntoView {
    // Auth Context State
    let (user, set_user) = signal::<Option<User>>(None);
    let (show_auth_modal, set_show_auth_modal) = signal(false);

    provide_context(AuthContext {
        user,
        set_user,
        show_auth_modal,
        set_show_auth_modal,
    });

    // Provide Toast Context
    components::toast::provide_toast_context();

    // Check for auth on mount
    Effect::new(move |_| {
        if let Some(auth) = load_auth() {
             set_user.set(Some(User {
                 username: auth.username.clone(),
                 is_guest: auth.is_guest,
             }));
        }
    });

    let is_dark_mode = RwSignal::new(false);

    // Initialize dark mode from local storage
    Effect::new(move |_| {
        if let Ok(Some(storage)) = window().local_storage() {
            if let Ok(Some(value)) = storage.get_item("dark_mode") {
                is_dark_mode.set(value == "true");
            }
        }
    });

    // Toggle dark mode class on html element
    Effect::new(move |_| {
        let is_dark = is_dark_mode.get();
        let doc = web_sys::window().unwrap().document().unwrap().document_element().unwrap();
        if is_dark {
            let _ = doc.class_list().add_1("dark");
        } else {
            let _ = doc.class_list().remove_1("dark");
        }

        if let Ok(Some(storage)) = window().local_storage() {
            let _ = storage.set_item("dark_mode", if is_dark { "true" } else { "false" });
        }
    });

    view! {
        <Router>
            <components::toast::ToastContainer />
            <div class="max-w-4xl mx-auto p-5 dark:text-gray-100 min-h-screen flex flex-col">
                <header class="flex justify-between items-center mb-8">
                    <h1 class="text-4xl font-bold text-blue-500">
                        <A href="/" attr:class="hover:text-blue-600 transition-colors">"文字"</A>
                    </h1>
                    <div class="flex items-center space-x-4">
                         <UserMenu />
                         <button
                            on:click=move |_| is_dark_mode.update(|d| *d = !*d)
                            class="p-2 rounded-full hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                            title="Toggle Dark Mode"
                        >
                            {move || if is_dark_mode.get() { "🌙" } else { "☀️" }}
                        </button>
                    </div>
                </header>

                <main class="flex-grow">
                    <Show when=move || show_auth_modal.get()>
                        <AuthModal
                            on_close=Callback::from(move || set_show_auth_modal.set(false))
                            on_success=Callback::from(move || set_show_auth_modal.set(false))
                        />
                    </Show>

                    <Routes fallback=|| "Not Found.">
                        <Route path=path!("/") view=Home />
                        <Route path=path!("/lobby/:id") view=LobbyPage/>
                    </Routes>
                </main>

                <footer class="text-center mt-8 pt-4 border-t border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm">
                    <p>"Learn Japanese Kanji through word recognition"</p>
                </footer>
            </div>
        </Router>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {}
