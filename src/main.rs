use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use csv::{Reader, StringRecord};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, sync::Arc};
use tower_http::services::ServeDir;

#[derive(Serialize)]
struct KanjiPrompt {
    kanji: String,
}

#[derive(Deserialize)]
struct UserInput {
    word: String,
    kanji: String,
}

fn vectorize_word_list(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut word_list = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let word: &str = record.get(0).unwrap_or("N/A");
        word_list.push(word.to_string());
    }

    Ok(word_list)
}

fn vectorize_joyo_kanji(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file: File = File::open(path)?;
    let mut rdr: Reader<File> = Reader::from_reader(file);
    let mut kanji_list = Vec::new();

    for result in rdr.records() {
        let record: StringRecord = result?;
        let kanji: &str = record.get(0).unwrap_or("N/A");
        kanji_list.push(kanji.to_string());
    }

    Ok(kanji_list)
}

fn is_valid_word(guess: &str, word_list: &[String]) -> bool {
    word_list.contains(&guess.to_string())
}

fn is_valid_kanji(guess: &str, kanji: &str) -> bool {
    guess.contains(kanji)
}

#[derive(Clone)]
struct AppState {
    word_list: Vec<String>,
    kanji_list: Vec<String>,
}

type SharedState = Arc<AppState>;

async fn get_kanji(State(state): State<SharedState>) -> impl IntoResponse {
    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..state.kanji_list.len());
    let kanji = &state.kanji_list[random_index];
    println!("Serving Kanji: {}", kanji);
    Json(KanjiPrompt {
        kanji: kanji.clone(),
    })
}

async fn check_word(
    State(state): State<SharedState>,
    Json(input): Json<UserInput>,
) -> impl IntoResponse {
    let word_list = &state.word_list;
    let input_word = input.word.trim();
    let input_kanji = input.kanji.trim();

    let good_kanji: bool = is_valid_kanji(&input_word, &input_kanji);
    let good_word: bool = is_valid_word(&input_word, word_list);
    let mut _cont: bool = true; //possible boolean use for discontinuing on incorrect guess

    let response = if good_kanji && good_word {
        "Good Guess!".to_string()
    } else if good_kanji && !good_word {
        "Bad Guess: Correct kanji, but not a valid word.".to_string()
    } else if !good_kanji && good_word {
        "Bad Guess: Valid word, but does not contain the correct kanji.".to_string()
    } else {
        "Bad guess: Incorrect kanji and not a valid word.".to_string()
    };

    response
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let word_list =
        vectorize_word_list("./data/kanji_words.csv").expect("Failed to load word list");
    let kanji_list =
        vectorize_joyo_kanji("./data/joyo_kanji.csv").expect("Failed to load kanji list");

    let app_state = AppState {
        word_list,
        kanji_list,
    };
    let shared_state = Arc::new(app_state);

    let app = Router::new()
        .route("/kanji", get(get_kanji))
        .route("/check_word", post(check_word))
        .with_state(shared_state)
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
