import React, { useState } from 'react';

// Interface matching the Rust API response from create_lobby and join_lobby
interface LobbyResponse {
  message?: string;
  lobby_id: string;
  error?: string;
}

interface LobbyComponentProps {
  onLobbyJoined: (lobbyId: string) => void;
}

const LobbyComponent: React.FC<LobbyComponentProps> = ({ onLobbyJoined }) => {
  const [inputLobbyId, setInputLobbyId] = useState('');
  const [status, setStatus] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const createLobby = async () => {
    setIsLoading(true);
    setStatus('Creating lobby...');

    try {
      const response = await fetch('/lobby/create', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      const data: LobbyResponse = await response.json();

      if (data.lobby_id) {
        setStatus(`Lobby created: ${data.lobby_id}`);
        onLobbyJoined(data.lobby_id);
      } else {
        setStatus('Error creating lobby');
      }
    } catch (error) {
      console.error('Failed to create lobby:', error);
      setStatus('Error connecting to server');
    } finally {
      setIsLoading(false);
    }
  };

  const joinLobby = async () => {
    if (!inputLobbyId.trim()) {
      setStatus('Please enter a lobby ID');
      return;
    }

    setIsLoading(true);
    setStatus(`Joining lobby ${inputLobbyId}...`);

    try {
      const response = await fetch(`/lobby/join/${inputLobbyId}`);

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      const data: LobbyResponse = await response.json();

      if (data.lobby_id) {
        setStatus(`Joined lobby: ${data.lobby_id}`);
        onLobbyJoined(data.lobby_id);
      } else if (data.error) {
        setStatus(`Error: ${data.error}`);
      } else {
        setStatus('Error joining lobby');
      }
    } catch (error) {
      console.error('Failed to join lobby:', error);
      setStatus('Error connecting to server');
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      joinLobby();
    }
  };

  return (
    <div className="lobby-container">
      <h2>Join or Create a Game</h2>

      <div className="lobby-actions">
        <button
          onClick={createLobby}
          disabled={isLoading}
          className="create-lobby-btn"
        >
          Create New Game
        </button>

        <div className="join-lobby">
          <input
            type="text"
            value={inputLobbyId}
            onChange={(e) => setInputLobbyId(e.target.value)}
            onKeyDown={handleKeyPress}
            placeholder="Enter Lobby ID"
            disabled={isLoading}
            className="lobby-input"
          />
          <button
            onClick={joinLobby}
            disabled={isLoading || !inputLobbyId.trim()}
            className="join-lobby-btn"
          >
            Join Game
          </button>
        </div>
      </div>

      {status && (
        <div className={`status-message ${status.includes('Error') ? 'error' : ''}`}>
          {status}
        </div>
      )}

      <div className="instructions">
        <h3>How to Play</h3>
        <p>Create a new game or join an existing one with a lobby ID.</p>
        <p>Once in a game, you'll be shown a kanji character.</p>
        <p>Type a Japanese word that contains that kanji and submit it to score points!</p>
      </div>
    </div>
  );
};

export default LobbyComponent;
