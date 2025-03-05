import { useState } from 'react';
import LobbyComponent from './components/LobbyComponent';
import GameComponent from './components/GameComponent';
import './styles/Components.css';

// Define TypeScript interfaces that match your Rust models
// interface KanjiPrompt {
//   kanji: string;
// }
//
// interface UserInput {
//   word: string;
//   kanji: string;
// }
//
// interface LobbyResponse {
//   message?: string;
//   lobby_id: string;
//   error?: string;
// }
//
// interface CheckWordResponse {
//   message: string;
//   score: number;
//   error?: string;
// }

function App() {
  const [lobbyId, setLobbyId] = useState<string>('');
  const [isInGame, setIsInGame] = useState<boolean>(false);

  const handleLobbyJoined = (newLobbyId: string) => {
    setLobbyId(newLobbyId);
    setIsInGame(true);
  };

  const handleExitGame = () => {
    setIsInGame(false);
    setLobbyId('');
  };

  return (
    <div className="app-container">
      <header>
        <h1>Kanji Guessing Game</h1>
      </header>

      <main>
        {!isInGame ? (
          <LobbyComponent onLobbyJoined={handleLobbyJoined} />
        ) : (
          <GameComponent lobbyId={lobbyId} onExitGame={handleExitGame} />
        )}
      </main>

      <footer>
        <p>Learn Japanese Kanji through word recognition</p>
      </footer>
    </div>
  );
}

export default App;
