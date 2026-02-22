use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::context::AuthContext;
use crate::persistence::clear_auth;


#[component]
pub fn UserMenu() -> impl IntoView {
    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");
    let (is_open, set_is_open) = signal(false);

    let toggle_menu = move |_| set_is_open.update(|v| *v = !*v);

    let handle_login = move |_| {
        auth_context.set_show_auth_modal.set(true);
    };

    let handle_logout  = move |_| {
        if let Some(user) = auth_context.user.get() {
            spawn_local(async move {
                let _ = shared::logout(user.username.clone()).await;
            });
        }

        clear_auth();
        auth_context.set_user.set(None);
        set_is_open.set(false);
    };

    view! {
        <div class="relative">
            {move || {
                match auth_context.user.get() {
                    Some(user) => view! {
                        <div class="relative">
                            <button 
                                class="flex items-center space-x-2 text-gray-700 dark:text-gray-200 hover:text-blue-600 font-medium transition-colors"
                                on:click=toggle_menu
                            >
                                <span>{user.username}</span>
                                <span class="text-xs">"▼"</span>
                            </button>
                            {move || is_open.get().then(|| view! {
                                <div class="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-100 dark:border-gray-700 py-1 z-50">
                                    <button
                                        class="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                                        on:click=handle_logout
                                    >
                                        "Sign Out"
                                    </button>
                                </div>
                            })}
                        </div>
                    }.into_any(),
                    None => view! {
                        <button
                            class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors text-sm shadow-sm"
                            on:click=handle_login
                        >
                            "Sign In"
                        </button>
                    }.into_any()
                }
            }}
        </div>
    }
}
