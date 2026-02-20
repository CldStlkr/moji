use leptos::prelude::*;

#[component]
pub fn GameFeedback(
    result: ReadSignal<String>,
    error_message: ReadSignal<String>,
) -> impl IntoView {
    let get_result_class = move || {
        let result_text = result.get();
        if result_text.is_empty() {
            return "".to_string();
        }
        let base_classes = "p-6 rounded-xl text-center font-bold text-lg border-2 transition-colors";
        if result_text.contains("Good guess") {
            format!(
                "{} bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-800 text-green-800 dark:text-green-300",
                base_classes
            )
        } else if result_text.contains("Bad") {
            format!("{} bg-red-50 dark:bg-red-900/30 border-red-200 dark:border-red-800 text-red-800 dark:text-red-300", base_classes)
        } else {
            format!("{} bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-300", base_classes)
        }
    };

    view! {
        <div class="space-y-4 mt-4">
             // Result Message
            <Show when=move || !result.get().is_empty()>
                <div class=get_result_class>{move || result.get()}</div>
            </Show>

            // Error Message
            <Show when=move || !error_message.get().is_empty()>
                <div class="p-4 rounded bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 text-center font-medium">
                    {move || error_message.get()}
                </div>
            </Show>

             <div class="mt-8 pt-6 border-t border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 text-sm">
                <p class="mb-2">"Type a Japanese word containing the displayed kanji."</p>
                <p>"Click \"Submit\" to check your answer."</p>
            </div>
        </div>
    }
}
