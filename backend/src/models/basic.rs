use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct KanjiPrompt {
    pub kanji: String,
}

#[derive(Deserialize)]
pub struct UserInput {
    pub word: String,
    pub kanji: String,
}

#[derive(Default)]
pub struct UserScore {
    pub score: u32,
}

impl UserScore {
    pub fn new() -> Self {
        Self { score: 0 }
    }
}
