/**
 * GameCard component displays a game with play/download button.
 */

import { useState, useEffect } from 'react';
import { GameInfo, isGameDownloaded, downloadGame, launchGame, DownloadProgress } from '../lib/api';
import { listen } from '@tauri-apps/api/event';
import './GameCard.css';

interface GameCardProps {
  game: GameInfo;
  onStatusChange?: (status: string) => void;
}

export function GameCard({ game, onStatusChange }: GameCardProps) {
  const [isDownloaded, setIsDownloaded] = useState<boolean | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);

  // Check if game is downloaded on mount
  useEffect(() => {
    async function checkDownloadStatus() {
      try {
        const downloaded = await isGameDownloaded(game.id);
        setIsDownloaded(downloaded);
      } catch (err) {
        console.error('Failed to check game status:', err);
        setIsDownloaded(false);
      }
    }
    checkDownloadStatus();
  }, [game.id]);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<DownloadProgress>('download-progress', (event) => {
      if (event.payload.item === game.id) {
        setProgress(event.payload.progress);

        if (event.payload.progress >= 100) {
          setIsDownloaded(true);
          setProgress(null);
          setIsLoading(false);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [game.id]);

  async function handlePlay() {
    if (isLoading) return;

    setIsLoading(true);
    onStatusChange?.(`Launching ${game.name}...`);

    try {
      if (!isDownloaded) {
        onStatusChange?.(`Downloading ${game.name}...`);
        await downloadGame(game.id);
        setIsDownloaded(true);
      }

      await launchGame(game.id);
      onStatusChange?.(`${game.name} launched`);
    } catch (err) {
      console.error('Failed to play game:', err);
      onStatusChange?.(`Error: ${err}`);
    } finally {
      setIsLoading(false);
      setProgress(null);
    }
  }

  async function handleDownload() {
    if (isLoading || isDownloaded) return;

    setIsLoading(true);
    onStatusChange?.(`Downloading ${game.name}...`);

    try {
      await downloadGame(game.id);
      setIsDownloaded(true);
      onStatusChange?.(`${game.name} downloaded`);
    } catch (err) {
      console.error('Failed to download game:', err);
      onStatusChange?.(`Error: ${err}`);
    } finally {
      setIsLoading(false);
      setProgress(null);
    }
  }

  const buttonText = isLoading
    ? progress !== null
      ? `${progress}%`
      : 'Loading...'
    : isDownloaded
    ? 'Play'
    : 'Download';

  return (
    <div className="game-card">
      <div className="game-card-content">
        <h3 className="game-title">{game.name}</h3>
        <p className="game-description">{game.description}</p>

        {progress !== null && (
          <div className="progress-bar-container">
            <div className="progress-bar" style={{ width: `${progress}%` }} />
          </div>
        )}

        <div className="game-actions">
          <button
            className={`game-button ${isDownloaded ? 'primary' : 'secondary'}`}
            onClick={isDownloaded ? handlePlay : handleDownload}
            disabled={isLoading || isDownloaded === null}
          >
            {buttonText}
          </button>
        </div>
      </div>
    </div>
  );
}
