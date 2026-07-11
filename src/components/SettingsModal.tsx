/**
 * SettingsModal component for app configuration.
 */

import { useState, useEffect } from 'react';
import {
  Settings,
  getSettings,
  saveSettings,
  checkFlashInstalled,
  getFlashPath,
  downloadFlash,
  checkRuffleInstalled,
  getRufflePath,
  downloadRuffle,
  getFlashInstallDate,
  getRuffleInstallDate,
  DownloadProgress,
} from '../lib/api';
import { listen } from '@tauri-apps/api/event';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import './SettingsModal.css';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onStatusChange?: (status: string) => void;
  onSoundToggle?: (enabled: boolean) => void;
}

export function SettingsModal({ isOpen, onClose, onStatusChange, onSoundToggle }: SettingsModalProps) {
  const [settings, setSettings] = useState<Settings>({});
  const [flashInstalled, setFlashInstalled] = useState<boolean | null>(null);
  const [ruffleInstalled, setRuffleInstalled] = useState<boolean | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  // The resolved on-disk paths (shown in the input when no custom path is set)
  const [resolvedFlashPath, setResolvedFlashPath] = useState('');
  const [resolvedRufflePath, setResolvedRufflePath] = useState('');
  const [flashInstallDate, setFlashInstallDate] = useState('');
  const [ruffleInstallDate, setRuffleInstallDate] = useState('');

  useEffect(() => {
    if (isOpen) {
      loadSettings();
      checkFlash();
      checkRuffle();
      // Fetch resolved default paths from Rust so inputs always show something
      getFlashPath().then(setResolvedFlashPath).catch(() => {});
      getRufflePath().then(setResolvedRufflePath).catch(() => {});
      getFlashInstallDate().then(setFlashInstallDate).catch(() => {});
      getRuffleInstallDate().then(setRuffleInstallDate).catch(() => {});
    }
  }, [isOpen]);

  // Listen for download progress
  useEffect(() => {
    const unlisten = listen<DownloadProgress>('download-progress', (event) => {
      if (event.payload.item === 'flash_player') {
        setDownloadProgress(event.payload.progress);

        if (event.payload.progress >= 100) {
          setFlashInstalled(true);
          setDownloadProgress(null);
          setIsDownloading(false);
        }
      } else if (event.payload.item === 'ruffle') {
        setDownloadProgress(event.payload.progress);

        if (event.payload.progress >= 100) {
          setRuffleInstalled(true);
          setDownloadProgress(null);
          setIsDownloading(false);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadSettings() {
    try {
      const s = await getSettings();
      setSettings(s);
    } catch (err) {
      console.error('Failed to load settings:', err);
    }
  }

  async function checkFlash() {
    try {
      const installed = await checkFlashInstalled();
      setFlashInstalled(installed);
    } catch (err) {
      console.error('Failed to check Flash:', err);
    }
  }

  async function checkRuffle() {
    try {
      const installed = await checkRuffleInstalled();
      setRuffleInstalled(installed);
    } catch (err) {
      console.error('Failed to check Ruffle:', err);
    }
  }

  async function handleDownloadFlash() {
    if (isDownloading) return;
    setIsDownloading(true);
    onStatusChange?.('Downloading Flash Player...');
    try {
      const installedPath = await downloadFlash();
      setFlashInstalled(true);
      setResolvedFlashPath(installedPath);
      getFlashInstallDate().then(setFlashInstallDate).catch(() => {});
      onStatusChange?.('Flash Player installed');
    } catch (err) {
      console.error('Failed to download Flash:', err);
      onStatusChange?.(`Error: ${err}`);
      setIsDownloading(false);
    } finally {
      setDownloadProgress(null);
    }
  }

  async function handleDownloadRuffle() {
    if (isDownloading) return;
    setIsDownloading(true);
    onStatusChange?.('Downloading Ruffle...');
    try {
      const installedPath = await downloadRuffle();
      setRuffleInstalled(true);
      setResolvedRufflePath(installedPath);
      getRuffleInstallDate().then(setRuffleInstallDate).catch(() => {});
      onStatusChange?.('Ruffle installed');
    } catch (err) {
      console.error('Failed to download Ruffle:', err);
      onStatusChange?.(`Error: ${err}`);
      setIsDownloading(false);
    } finally {
      setDownloadProgress(null);
    }
  }

  async function handleSave() {
    try {
      await saveSettings(settings);
      onStatusChange?.('Settings saved');
      onClose();
    } catch (err) {
      console.error('Failed to save settings:', err);
      onStatusChange?.(`Error: ${err}`);
    }
  }

  /** ISO 8601 string'i kullanıcı dostu tarihe çevirir. */
  function formatInstallDate(iso: string): string {
    if (!iso) return '';
    try {
      return new Date(iso).toLocaleString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return iso;
    }
  }

  if (!isOpen) return null;

  return (
    <div
      className="modal-overlay"
      onClick={(e) => {
        // Close only when clicking directly on the overlay (not children)
        if (e.currentTarget === e.target) onClose();
      }}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClose();
        }
      }}
    >
      <div className="modal-content">
        <div className="modal-header">
          <h2>Settings</h2>
          <button className="close-button" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="modal-body">
          <div className="settings-section">
            <h3>Player Preference</h3>
            <div className="player-toggle">
              <label className={!settings.use_ruffle ? 'selected' : ''}>
                <input
                  type="radio"
                  name="player"
                  checked={!settings.use_ruffle}
                  onChange={() => setSettings({ ...settings, use_ruffle: false })}
                />
                Adobe Flash Player
              </label>
              <label className={settings.use_ruffle ? 'selected' : ''}>
                <input
                  type="radio"
                  name="player"
                  checked={!!settings.use_ruffle}
                  onChange={() => setSettings({ ...settings, use_ruffle: true })}
                />
                Ruffle
              </label>
            </div>
          </div>

          {!settings.use_ruffle ? (
            <div className="settings-section">
              <h3>Flash Player</h3>
              <div className="flash-status">
                <span
                  className={`status-indicator ${flashInstalled ? 'installed' : 'not-installed'}`}
                >
                  {flashInstalled === null
                    ? 'Checking...'
                    : flashInstalled
                    ? '✓ Installed'
                    : '✗ Not installed'}
                </span>

                {!flashInstalled ? (
                  <button
                    className="download-flash-button"
                    onClick={handleDownloadFlash}
                    disabled={isDownloading}
                  >
                    {isDownloading
                      ? downloadProgress !== null
                        ? `${downloadProgress}%`
                        : 'Downloading...'
                      : 'Download Flash Player'}
                  </button>
                ) : (
                  <button
                    className="update-button"
                    onClick={handleDownloadFlash}
                    disabled={isDownloading}
                    title="Re-download / update Flash Player"
                  >
                    {isDownloading
                      ? downloadProgress !== null
                        ? `${downloadProgress}%`
                        : 'Updating...'
                      : '↻ Update'}
                  </button>
                )}
              </div>

              {flashInstallDate && (
                <div className="install-date">
                  Installed: {formatInstallDate(flashInstallDate)}
                </div>
              )}

              {downloadProgress !== null && (
                <div className="progress-bar-container">
                  <div className="progress-bar" style={{ width: `${downloadProgress}%` }} />
                </div>
              )}
            </div>
          ) : (
            <div className="settings-section">
              <h3>Ruffle</h3>
              <div className="flash-status">
                <span
                  className={`status-indicator ${ruffleInstalled ? 'installed' : 'not-installed'}`}
                >
                  {ruffleInstalled === null
                    ? 'Checking...'
                    : ruffleInstalled
                    ? '✓ Installed'
                    : '✗ Not installed'}
                </span>

                {!ruffleInstalled ? (
                  <button
                    className="download-flash-button"
                    onClick={handleDownloadRuffle}
                    disabled={isDownloading}
                  >
                    {isDownloading
                      ? downloadProgress !== null
                        ? `${downloadProgress}%`
                        : 'Downloading...'
                      : 'Download Ruffle'}
                  </button>
                ) : (
                  <button
                    className="update-button"
                    onClick={handleDownloadRuffle}
                    disabled={isDownloading}
                    title="Re-download / update Ruffle"
                  >
                    {isDownloading
                      ? downloadProgress !== null
                        ? `${downloadProgress}%`
                        : 'Updating...'
                      : '↻ Update'}
                  </button>
                )}
              </div>

              {ruffleInstallDate && (
                <div className="install-date">
                  Installed: {formatInstallDate(ruffleInstallDate)}
                </div>
              )}

              {downloadProgress !== null && (
                <div className="progress-bar-container">
                  <div className="progress-bar" style={{ width: `${downloadProgress}%` }} />
                </div>
              )}
            </div>
          )}

          <div className="settings-section">
            <h3>Custom {settings.use_ruffle ? 'Ruffle' : 'Flash'} Path</h3>
            <div className="path-row">
              <input
                type="text"
                className="settings-input"
                placeholder={settings.use_ruffle ? resolvedRufflePath || 'Default path' : resolvedFlashPath || 'Default path'}
                value={
                  settings.use_ruffle
                    ? (settings.ruffle_path ?? '')
                    : (settings.flash_player_path ?? '')
                }
                onChange={(e) => {
                  const val = e.target.value || undefined;
                  if (settings.use_ruffle) {
                    setSettings({ ...settings, ruffle_path: val });
                  } else {
                    setSettings({ ...settings, flash_player_path: val });
                  }
                }}
              />
              <button
                className="browse-button"
                onClick={async () => {
                  const selected = await dialogOpen({
                    multiple: false,
                    filters: settings.use_ruffle
                      ? [{ name: 'Executable', extensions: ['exe', 'app', ''] }]
                      : [{ name: 'Flash Player', extensions: ['exe', 'app', ''] }],
                  });
                  if (typeof selected === 'string' && selected) {
                    if (settings.use_ruffle) {
                      setSettings({ ...settings, ruffle_path: selected });
                    } else {
                      setSettings({ ...settings, flash_player_path: selected });
                    }
                  }
                }}
              >
                Browse...
              </button>
            </div>
            {/* Show the resolved default path as a hint when no custom path is set */}
            {!(settings.use_ruffle ? settings.ruffle_path : settings.flash_player_path) && (
              <div className="path-hint">
                Default: {settings.use_ruffle ? resolvedRufflePath || '—' : resolvedFlashPath || '—'}
              </div>
            )}
          </div>

          <div className="settings-section">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={settings.sound_enabled ?? true}
                onChange={(e) => {
                  const enabled = e.target.checked;
                  setSettings({ ...settings, sound_enabled: enabled });
                  onSoundToggle?.(enabled);
                }}
              />
              <span>Enable sounds</span>
            </label>
          </div>
        </div>

        <div className="modal-footer">
          <button className="cancel-button" onClick={onClose}>
            Cancel
          </button>
          <button className="save-button" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
