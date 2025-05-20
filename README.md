# 文字 (Moji) - Kanji Learning Game

A real-time multiplayer web application for learning Japanese kanji through interactive gameplay. Built with Rust using Axum for the backend API and Leptos for the WebAssembly frontend.

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)

## Overview

文字 (Moji) is a web-based game designed to help users learn Japanese kanji characters through active recognition. Players are presented with a kanji character and must submit valid Japanese words containing that character. The game supports multiplayer lobbies where players can compete and learn together.

## Features

- **Real-time gameplay**: Test your knowledge of Japanese kanji characters
- **Multiplayer support**: Create and join game lobbies with friends
- **Word validation**: Automatic validation against a dictionary of Japanese words
- **Score tracking**: Keep track of correct answers and compete with other players
- **Responsive design**: Play on desktop or mobile devices

## Technology Stack

- **Backend**: Rust + Axum web framework
- **Frontend**: Rust + Leptos (WebAssembly)
- **Database**: PostgreSQL with SQLx
- **Containerization**: Docker support for easy deployment

## Getting Started

### Prerequisites

- Rust toolchain (1.70 or newer)
- PostgreSQL database
- Wasm target: `rustup target add wasm32-unknown-unknown`
- Trunk: `cargo install trunk`

### Local Development

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/kanji-guesser.git
   cd kanji-guesser
   ```

2. Set up the database:
   ```
   export DATABASE_URL=postgres://user:password@localhost/kanji_guesser
   ```

3. Run database migrations:
   ```
   cd backend
   cargo run --bin migrate
   ```

4. Start the backend server:
   ```
   cd backend
   cargo run
   ```

5. In a separate terminal, start the frontend development server:
   ```
   cd frontend
   trunk serve
   ```

6. Visit `http://localhost:8080` in your browser to use the application

### Docker Deployment

```
docker-compose up -d
```

## Game Rules

1. Join or create a game lobby
2. When it's your turn, you'll be presented with a kanji character
3. Type a Japanese word containing that kanji character
4. Submit your answer to earn points if correct
5. Challenge other players and learn new vocabulary together

## Project Structure

- `/backend` - Axum API server
  - `/src/models` - Database models and business logic
  - `/src/api` - API endpoint handlers
  - `/src/db` - Database connection and utilities
  - `/src/error` - Error handling
  - `/src/data` - Data loading and processing

- `/frontend` - Leptos WebAssembly application
  - `/src/components` - UI components
  - `/src/api` - API client code
  - `/src/error` - Error handling

## Roadmap

### In Progress

- **Pre-game Lobby Enhancements**:
  - Lobby leader controls for game settings
  - Configurable kanji difficulty levels (N5-N1)
  - Customizable time limits for guesses
  - Player queue system with turn-based gameplay
  - Option to attempt previous player's missed kanji

- **Real-time Features**:
  - WebSocket integration for live player updates
  - Real-time typing indicators
  - Instant score updates

### Planned

- **Authentication & Accounts**:
  - User registration and login
  - Persistent player statistics
  - Achievement system

- **Enhanced Gameplay**:
  - Global leaderboards
  - Different game modes
  - Kanji information and learning resources

- **UI Improvements**:
  - Dark mode toggle
  - Language switching between Japanese and English
  - Mobile optimizations
  - Accessibility enhancements

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgements

- [Joyo Kanji List](https://www.bunka.go.jp/kokugo_nihongo/sisaku/joho/joho/kijun/naikaku/kanji/)
- [Japanese Word Dictionary](https://www.edrdg.org/jmdict/j_jmdict.html)
- [Leptos Framework](https://github.com/leptos-rs/leptos)
- [Axum Framework](https://github.com/tokio-rs/axum)
