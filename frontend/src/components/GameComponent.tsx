import React, { useState, useEffect, useRef } from 'react';

// Interfaces matching your Rust models
interface KanjiPrompt {
  kanji: string;
}

interface UserInput {
  word: string;
  kanji: string;
}

interface CheckWordResponse {
  message: string;
  score: number;
  error?: string;
}

interface GameComponentProps {
  lobbyId: string;
  onExitGame?: () => void;
}

const GameComponent: React.FC<GameComponentProps> = ({ lobbyId, onExitGame }) => {
  const [kanji, setKanji] = useState<string>('');
  const [word, setWord] = useState<string>('');
  const [result, setResult] = useState<string>('');
  const [score, setScore] = useState<number>(0);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string>('');

  const inputRef = useRef<HTMLInputElement>(null);

  // Fetch initial kanji when the component mounts
  useEffect(() => {
    getNewKanji();

    // Focus the input field
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  const getNewKanji = async () => {
    setIsLoading(true);
    setErrorMessage('');
    setResult('');

    try {
      const response = await fetch(`/kanji/${lobbyId}`);

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      const data: KanjiPrompt | { error: string } = await response.json();

      if ('kanji' in data) {
        setKanji(data.kanji);
      } else if ('error' in data) {
        setErrorMessage(data.error);
      }
    } catch (error) {
      console.error('Failed to fetch kanji:', error);
      setErrorMessage('Could not connect to the server. Please try again.');
    } finally {
      setIsLoading(false);
      // Focus on the input field after loading new kanji
      if (inputRef.current) {
        inputRef.current.focus();
      }
    }
  };


  const submitWord = async () => {
    if (!word.trim() || isLoading || !kanji) return;

    setIsLoading(true);
    setErrorMessage('');

    try {
      const userInput: UserInput = {
        word: word.trim(),
        kanji: kanji
      };

      const response = await fetch(`/check_word/${lobbyId}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(userInput)
      });

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      const data: CheckWordResponse | { error: string } = await response.json();

      if ('message' in data && 'score' in data) {
        setResult(data.message);
        setScore(data.score);
      } else if ('error' in data) {
        setErrorMessage(data.error);
      }
    } catch (error) {
      console.error('Failed to submit word:', error);
      setErrorMessage('Could not connect to the server. Please try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const handleNewKanji = async () => {
    setIsLoading(true);
    setErrorMessage('');
    setResult('');

    try {
      const response = await fetch(`/new_kanji/${lobbyId}`, {
        method: 'POST'
      });

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      const data: KanjiPrompt | { error: string } = await response.json();

      if ('kanji' in data) {
        setKanji(data.kanji);
      } else if ('error' in data) {
        setErrorMessage(data.error);
      }
    } catch (error) {
      console.error('Failed to fetch new kanji:', error);
      setErrorMessage('Could not connect to the server. Please try again.');
    } finally {
      setIsLoading(false);
      // Focus on the input field after loading new kanji
      if (inputRef.current) {
        inputRef.current.focus();
      }
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !isLoading) {
      submitWord();
    }
  };

  // Determine result message styling
  const getResultClassName = () => {
    if (!result) return '';

    if (result.includes('Good guess')) {
      return 'result-message correct';
    } else if (result.includes('Bad')) {
      return 'result-message incorrect';
    }

    return 'result-message';
  };

  return (
    <div className="game-container">
      <div className="game-header">
        <h2>Kanji Game</h2>
        <div className="score-display">Score: {score}</div>
        {onExitGame && (
          <button onClick={onExitGame} className="exit-game-btn">
            Exit Game
          </button>
        )}
      </div>

      <div className="lobby-info">
        Lobby ID: <span className="lobby-id">{lobbyId}</span>
        <button
          onClick={() => { navigator.clipboard.writeText(lobbyId) }}
          className="copy-btn"
          title="Copy Lobby ID"
        >
          Copy
        </button>
      </div>

      <div className="game-area">
        <div className="kanji-display">
          {isLoading ? (
            <div className="loading">Loading...</div>
          ) : (
            <div className="kanji">{kanji}</div>
          )}
        </div>

        <div className="input-area">
          <input
            ref={inputRef}
            type="text"
            value={word}
            onChange={(e) => setWord(e.target.value)}
            onKeyDown={handleKeyPress}
            placeholder="Enter a Japanese word with this kanji"
            disabled={isLoading}
            className="word-input"
          />

          <div className="game-buttons">
            <button
              onClick={submitWord}
              disabled={isLoading || !word.trim() || !kanji}
              className="submit-btn"
            >
              Submit
            </button>

            <button
              onClick={handleNewKanji}
              disabled={isLoading}
              className="new-kanji-btn"
            >
              New Kanji
            </button>
          </div>
        </div>

        {result && (
          <div className={getResultClassName()}>
            {result}
          </div>
        )}

        {errorMessage && (
          <div className="error-message">
            {errorMessage}
          </div>
        )}
      </div>

      <div className="game-instructions">
        <p>Type a Japanese word containing the displayed kanji.</p>
        <p>Click "Submit" to check your answer or "New Kanji" to get a different character.</p>
      </div>
    </div>
  );
};

export default GameComponent;
