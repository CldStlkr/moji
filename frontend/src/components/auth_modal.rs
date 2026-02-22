use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use shared::{check_username, authenticate, AuthRequest};
use crate::context::{AuthContext, User};
use crate::persistence::{save_auth, AuthData};

#[component]
pub fn AuthModal(
    on_close: Callback<()>,
    on_success: Callback<()>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let (stage, set_stage) = signal("username".to_string());

    let auth_context = use_context::<AuthContext>().expect("AuthContext missing");

    let finalize_auth = move |username: String, is_guest: bool| {
        let user = User {
            username: username.clone(),
            is_guest,
        };
        auth_context.set_user.set(Some(user));

        save_auth(&AuthData {
            username,
            is_guest,
        });

        auth_context.set_show_auth_modal.set(false);
        on_success.run(());
    };

    // --- Logic Handlers ---

    let submit_check_username = move || {
        let name = username.get();
        if name.is_empty() {
             set_error.set("Please enter a username".to_string());
             return;
        }

        spawn_local(async move {
            match check_username(name).await {
                Ok(res) => {
                    let available = res["available"].as_bool().unwrap_or(false);
                    let is_guest = res["is_guest"].as_bool().unwrap_or(false);

                    if available {
                        set_stage.set("guest_or_login".to_string());
                        set_error.set(String::new());
                    } else if is_guest {
                        set_error.set("Username already taken".to_string());
                    } else {
                        set_stage.set("password_login".to_string());
                        set_error.set(String::new());
                    }
                }
                Err(e) => set_error.set(format!("Error: {}", e)),
            }
        });
    };

    let submit_login = move || {
        let name = username.get();
        let pass = password.get();

        spawn_local(async move {
            let req = AuthRequest {
                username: name.clone(),
                password: Some(pass),
                create_guest: false,
            };

            match authenticate(req).await {
                Ok(_) => finalize_auth(name, false),
                Err(e) => set_error.set(format!("Login failed: {}", e)),
            }
        });
    };

    let submit_guest = move || {
         let name = username.get();
         spawn_local(async move {
             match crate::context::create_guest_account(name.clone()).await {
                 Ok(final_name) => finalize_auth(final_name, true),
                 Err(e) => set_error.set(format!("Guest login failed: {}", e)),
             }
         });
    };

    let submit_register = move || {
        let name = username.get();
        let pass = password.get();

        spawn_local(async move {
            let req = AuthRequest {
                username: name.clone(),
                password: Some(pass),
                create_guest: false,
            };
            match authenticate(req).await {
                 Ok(_) => finalize_auth(name, false),
                 Err(e) => set_error.set(format!("Registration failed: {}", e)),
            }
        });
    };

    let go_to_register = move || set_stage.set("password_register".to_string());
    let go_back = move || set_stage.set("guest_or_login".to_string());
    let change_user = move || set_stage.set("username".to_string());

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div class="bg-white dark:bg-gray-800 p-8 rounded-xl shadow-2xl max-w-sm w-full mx-4 border border-gray-200 dark:border-gray-700">
                <div class="flex justify-end">
                    <button
                        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 text-xl font-bold"
                        on:click=move |_| on_close.run(())
                    >
                        "✕"
                    </button>
                </div>
                <h2 class="text-2xl font-bold mb-6 text-gray-900 dark:text-white text-center">
                    {move || match stage.get().as_str() {
                        "username" => "Welcome",
                        "guest_or_login" => "Choose Access",
                        "password_login" => "Welcome Back",
                        "password_register" => "Create Account",
                        _ => "Auth"
                    }}
                </h2>

                <Show when=move || !error.get().is_empty()>
                    <div class="bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 p-3 rounded mb-4 text-sm">
                        {move || error.get()}
                    </div>
                </Show>

                // 1: ENTER USERNAME
                {move || (stage.get() == "username").then(|| view! {
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Username</label>
                            <input
                                type="text"
                                class="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 outline-none"
                                prop:value=move || username.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                    set_username.set(input.value());
                                }
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        submit_check_username();
                                    }
                                }
                                placeholder="Enter your name"
                            />
                        </div>
                        <button
                            class="w-full py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                            on:click=move |_| submit_check_username()
                        >
                            "Continue"
                        </button>
                    </div>
                })}

                // 2: GUEST OR LOGIN
                {move || (stage.get() == "guest_or_login").then(|| view! {
                    <div class="space-y-3">
                         <p class="text-gray-600 dark:text-gray-400 text-sm mb-4">
                            "The username " <span class="font-bold text-gray-900 dark:text-white">{username.get()}</span> " is available!"
                         </p>
                         <button
                            class="w-full py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg font-medium transition-colors"
                            on:click=move |_| submit_guest()
                        >
                            "Continue as Guest"
                        </button>
                        <div class="relative py-2">
                            <div class="absolute inset-0 flex items-center"><span class="w-full border-t border-gray-300 dark:border-gray-600"></span></div>
                            <div class="relative flex justify-center text-sm"><span class="px-2 bg-white dark:bg-gray-800 text-gray-500">or</span></div>
                        </div>
                        <button
                            class="w-full py-2 bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-white rounded-lg font-medium transition-colors"
                            on:click=move |_| go_to_register()
                        >
                            "Create Account"
                        </button>
                    </div>
                })}

                // 3: PASSWORD LOGIN
                {move || (stage.get() == "password_login").then(|| view! {
                    <div class="space-y-4">
                         <div class="flex items-center justify-between mb-2">
                             <p class="text-sm text-gray-600 dark:text-gray-400">Not you?</p>
                             <button class="text-sm text-blue-500 hover:underline" on:click=move |_| change_user()>"Change user"</button>
                         </div>
                         <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Password</label>
                            <input
                                type="password"
                                class="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 outline-none"
                                prop:value=move || password.get()
                                on:input=move |ev| {
                                     let target = ev.target().unwrap();
                                     let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                     set_password.set(input.value());
                                }
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        submit_login();
                                    }
                                }
                            />
                        </div>
                        <button
                            class="w-full py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                            on:click=move |_| submit_login()
                        >
                            "Sign In"
                        </button>
                    </div>
                })}

                // 4: REGISTER PASSWORD
                {move || (stage.get() == "password_register").then(|| view! {
                    <div class="space-y-4">
                         <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Create Password</label>
                            <input
                                type="password"
                                class="w-full px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 outline-none"
                                prop:value=move || password.get()
                                on:input=move |ev| {
                                     let target = ev.target().unwrap();
                                     let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                     set_password.set(input.value());
                                }
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        submit_register();
                                    }
                                }
                            />
                        </div>
                        <button
                            class="w-full py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg font-medium transition-colors"
                            on:click=move |_| submit_register()
                        >
                            "Sign Up"
                        </button>
                        <button 
                            class="w-full mt-2 text-sm text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                            on:click=move |_| go_back()
                        >
                            "Back"
                        </button>
                    </div>
                })}
            </div>
        </div>
    }
}
