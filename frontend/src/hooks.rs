use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::error::{get_user_friendly_message, log_error, ClientError};
use std::future::Future;
use std::pin::Pin;

/// A hook to handle API actions with loading state and error reporting.
///
/// Returns a tuple of (is_loading, status, action_callback)
type ApiAction = Pin<Box<dyn Future<Output = Result<(), ClientError>> + Send>>;

pub fn use_api_action(
    set_is_loading: WriteSignal<bool>,
    set_status: WriteSignal<String>,
) -> impl Fn(ApiAction) + Copy + Send + Sync + 'static
{
    move |action_fut| {
        spawn_local(async move {
            set_is_loading.set(true);

            match action_fut.await {
                Ok(_) => {
                    set_status.set(String::new());
                }
                Err(e) => {
                    log_error("API Action failed", e.clone());
                    set_status.set(get_user_friendly_message(e));
                }
            }

            set_is_loading.set(false);
        });
    }
}
