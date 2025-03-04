// Types that match Rust models
export interface KanjiPrompt {
  kanji: string;
}

export interface UserInput {
  word: string;
  kanji: string;
}

export interface LobbyResponse {
  message?: string;
  lobby_id: string;
  error?: string;
}

export interface CheckWordResponse {
  message: string;
  score: number;
  error?: string;
}
