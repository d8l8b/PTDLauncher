/**
 * useSounds — MP3-based sound effects for PTD Launcher.
 *
 * Uses HTMLAudioElement so Vite-bundled asset URLs work directly in Tauri.
 *
 * sound_enabled is read fresh from Rust on every play() call so that
 * toggling the checkbox in Settings takes effect immediately without
 * needing to restart the app.
 */

import { useCallback } from 'react';
import { getSettings } from '../lib/api';

import onSfx       from '../assets/on.mp3';
import openTabSfx  from '../assets/opentab.mp3';
import offSfx      from '../assets/off.mp3';
import closeTabSfx from '../assets/closetab.mp3';

export interface Sounds {
  launch: () => void;
  pokecenter: () => void;
  settingsOpen: () => void;
  off: () => void;
  closeTab: () => void;
}

export function useSounds(): Sounds {
  const play = useCallback((src: string) => {
    // Read sound_enabled fresh each time so Settings changes take effect instantly.
    getSettings()
      .then((s) => {
        if (s.sound_enabled === false) return;
        const audio = new Audio(src);
        audio.volume = 0.8;
        audio.play().catch(() => {});
      })
      .catch(() => {
        // If settings can't be read, play anyway (fail-open).
        const audio = new Audio(src);
        audio.volume = 0.8;
        audio.play().catch(() => {});
      });
  }, []);

  return {
    launch:       () => play(onSfx),
    pokecenter:   () => play(openTabSfx),
    settingsOpen: () => play(openTabSfx),
    off:          () => play(offSfx),
    closeTab:     () => play(closeTabSfx),
  };
}
