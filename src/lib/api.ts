import { invoke } from '@tauri-apps/api/core';

export interface Settings {
  flash_player_path?: string;
  use_ruffle?: boolean;
  ruffle_path?: string;
  sound_enabled?: boolean;
}

export interface DownloadProgress {
  item: string;
  progress: number;
  downloaded: number;
  total: number;
  status: string;
}

export type GameId = 'PTD1' | 'PTD1_Hacked' | 'PTD2' | 'PTD2_Hacked' | 'PTD3' | 'PTD3_Hacked';


/** Game metadata */
export interface GameInfo {
  id: GameId;
  name: string;
  description: string;
}

export const GAMES: GameInfo[] = [
  { id: 'PTD1', name: 'PTD 1', description: 'Pokemon Tower Defense' },
  {
    id: 'PTD1_Hacked',
    name: 'PTD 1 Hacked',
    description: 'Pokemon Tower Defense (Hacked)',
  },
  { id: 'PTD2', name: 'PTD 2', description: 'Pokemon Tower Defense 2' },
  {
    id: 'PTD2_Hacked',
    name: 'PTD 2 Hacked',
    description: 'Pokemon Tower Defense 2 (Hacked)',
  },
  { id: 'PTD3', name: 'PTD 3', description: 'Pokemon Tower Defense 3' },
  {
    id: 'PTD3_Hacked',
    name: 'PTD 3 Hacked',
    description: 'Pokemon Tower Defense 3 (Hacked)',
  },
];

// Flash Player commands

export async function checkFlashInstalled(): Promise<boolean> {
  return invoke<boolean>('check_flash_installed');
}

export async function getFlashPath(): Promise<string> {
  return invoke<string>('get_flash_path');
}

export async function downloadFlash(): Promise<string> {
  return invoke<string>('download_flash');
}

// Ruffle commands

export async function checkRuffleInstalled(): Promise<boolean> {
  return invoke<boolean>('check_ruffle_installed');
}

export async function getRufflePath(): Promise<string> {
  return invoke<string>('get_ruffle_path');
}

export async function downloadRuffle(): Promise<string> {
  return invoke<string>('download_ruffle');
}

// Game commands

export async function isGameDownloaded(gameId: GameId): Promise<boolean> {
  return invoke<boolean>('is_game_downloaded', { gameId });
}

export async function getGamePath(gameId: GameId): Promise<string | null> {
  return invoke<string | null>('get_game_path', { gameId });
}

export async function downloadGame(gameId: GameId): Promise<string> {
  return invoke<string>('download_game', { gameId });
}

export async function launchGame(gameId: GameId): Promise<void> {
  return invoke<void>('launch_game', { gameId });
}

// Install date commands

export async function getFlashInstallDate(): Promise<string> {
  return invoke<string>('get_flash_install_date');
}

export async function getRuffleInstallDate(): Promise<string> {
  return invoke<string>('get_ruffle_install_date');
}

// Settings commands

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings');
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke<void>('save_settings', { newSettings: settings });
}

// Update commands

/**
 * Returns an array of game IDs that have a newer version available on the
 * server (or have never been downloaded).  An empty array means everything
 * is up-to-date.
 */
export async function checkForUpdates(): Promise<GameId[]> {
  return invoke<GameId[]>('check_for_updates');
}
