import { useState, useEffect, useCallback } from 'react';
import './App.css';
import { SettingsModal } from './components/SettingsModal';
import { ImageButton } from './components/ImageButton';
import { GAMES, isGameDownloaded, downloadGame, launchGame, checkForUpdates } from './lib/api';
import type { GameId } from './lib/api';
import { openUrl } from '@tauri-apps/plugin-opener';
import { MESSAGES, PTD_URLS, ALTS } from './lib/constants';
import { listen } from '@tauri-apps/api/event';
import { useSounds } from './hooks/useSounds';

// Header assets
import logo from './assets/logo.png';
import settingsIcon from './assets/settings.png';
import updateIcon from './assets/update.png';

// Button sprite assets
import pcDefault  from './assets/PTD_PC_DEFAULT.png';
import pcHover    from './assets/PTD_PC_HOVER.png';
import pcPressed  from './assets/PTD_PC_PRESSED.png';

import playDefault  from './assets/PTD_PLAY_DEFAULT.png';
import playHover    from './assets/PTD_PLAY_HOVER.png';
import playPressed  from './assets/PTD_PLAY_PRESSED.png';

import hackedDefault  from './assets/PTD_HACKED_DEFAULT.png';
import hackedHover    from './assets/PTD_HACKED_HOVER.png';
import hackedPressed  from './assets/PTD_HACKED_PRESSED.png';

// ─── Types ────────────────────────────────────────────────────

interface ButtonState {
  downloaded: boolean | null;
  loading: boolean;
  progress: number | null;
}

type ButtonStates = Record<GameId, ButtonState>;

const defaultState = (): ButtonState => ({ downloaded: null, loading: false, progress: null });

// ─── Component ────────────────────────────────────────────────

function App() {
  const [statusMessage, setStatusMessage] = useState<string>(MESSAGES.NO_UPDATES);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updateList, setUpdateList] = useState<GameId[]>([]);
  const [updateModalOpen, setUpdateModalOpen] = useState(false);
  const [updatingIds, setUpdatingIds] = useState<Set<GameId>>(new Set());

  const sounds = useSounds();

  const [btnStates, setBtnStates] = useState<ButtonStates>({
    PTD1:        defaultState(),
    PTD1_Hacked: defaultState(),
    PTD2:        defaultState(),
    PTD2_Hacked: defaultState(),
    PTD3:        defaultState(),
    PTD3_Hacked: defaultState(),
  });

  // ── Check download status on mount ───────────────────────
  useEffect(() => {
    GAMES.forEach(async (game) => {
      try {
        const downloaded = await isGameDownloaded(game.id);
        setBtnStates((prev) => ({ ...prev, [game.id]: { ...prev[game.id], downloaded } }));
      } catch {
        setBtnStates((prev) => ({ ...prev, [game.id]: { ...prev[game.id], downloaded: false } }));
      }
    });
  }, []);

  // ── Update checker ────────────────────────────────────────
  const runUpdateCheck = useCallback(async () => {
    if (checkingUpdates) return;
    setCheckingUpdates(true);
    setStatusMessage('Checking for updates...');
    try {
      const stale = await checkForUpdates();
      if (stale.length === 0) {
        setStatusMessage(MESSAGES.NO_UPDATES);
      } else {
        setStatusMessage(`${stale.length} update(s) available`);
        setUpdateList(stale);
        setUpdateModalOpen(true);
      }
    } catch {
      setStatusMessage(MESSAGES.NO_UPDATES);
    } finally {
      setCheckingUpdates(false);
    }
  }, [checkingUpdates]);

  // Run automatically on mount
  useEffect(() => {
    runUpdateCheck();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ── Download progress listener ────────────────────────────
  useEffect(() => {
    const unlisten = listen<{ item: string; progress: number }>('download-progress', (event) => {
      const id = event.payload.item as GameId;
      const progress = event.payload.progress;

      setBtnStates((prev) => {
        if (!(id in prev)) return prev;
        if (progress >= 100) {
          return { ...prev, [id]: { downloaded: true, loading: false, progress: null } };
        }
        return { ...prev, [id]: { ...prev[id], progress } };
      });
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // ── Update all / individual game download ────────────────

  async function handleUpdateGame(id: GameId) {
    if (updatingIds.has(id)) return;
    setUpdatingIds((prev) => new Set(prev).add(id));
    setStatusMessage(`Downloading ${id}...`);
    try {
      await downloadGame(id);
      setGameState(id, { downloaded: true });
      setUpdateList((prev) => prev.filter((x) => x !== id));
      setStatusMessage(`${id} updated`);
    } catch (err) {
      setStatusMessage(`Error: ${err}`);
    } finally {
      setUpdatingIds((prev) => { const s = new Set(prev); s.delete(id); return s; });
    }
  }

  async function handleUpdateAll() {
    // Use the fixed display order instead of the arbitrary server-returned order
    const ordered = (['PTD1', 'PTD1_Hacked', 'PTD2', 'PTD2_Hacked', 'PTD3', 'PTD3_Hacked'] as GameId[])
      .filter((id) => updateList.includes(id));
    for (const id of ordered) {
      await handleUpdateGame(id);
    }
    setUpdateModalOpen(false);
  }

  function setGameState(id: GameId, patch: Partial<ButtonState>) {
    setBtnStates((prev) => ({ ...prev, [id]: { ...prev[id], ...patch } }));
  }

  async function handlePlay(id: GameId, name: string) {
    const state = btnStates[id];
    if (state.loading) return;

    setGameState(id, { loading: true });

    try {
      if (!state.downloaded) {
        setStatusMessage(`Downloading ${name}...`);
        await downloadGame(id);
        setGameState(id, { downloaded: true });
      }
      setStatusMessage(`Launching ${name}...`);
      sounds.launch();
      await launchGame(id);
      setStatusMessage(`${name} launched`);
    } catch (err) {
      setStatusMessage(`Error: ${err}`);
    } finally {
      setGameState(id, { loading: false, progress: null });
    }
  }

  function handlePokecenter(version: '1' | '2' | '3') {
    sounds.pokecenter();
    const urls: Record<string, string> = {
      '1': PTD_URLS.PTD1,
      '2': PTD_URLS.PTD2,
      '3': PTD_URLS.PTD3,
    };
    openUrl(urls[version]);
    setStatusMessage(MESSAGES.OPENED_POKECENTER(version));
  }

  // ── Render ────────────────────────────────────────────────

  return (
    <div className="app">
      {/* ── Header ── */}
      <header className="app-header">
        <img src={logo} alt={ALTS.LOGO} className="header-logo" />
        <div className="header-actions">
          <button
            className={`icon-button${checkingUpdates ? ' spinning' : ''}`}
            title="Check for updates"
            onClick={runUpdateCheck}
            disabled={checkingUpdates}
          >
            <img src={updateIcon} alt="Check for updates" />
          </button>
          <button
            className="icon-button"
            onClick={() => { sounds.settingsOpen(); setSettingsOpen(true); }}
            title={ALTS.SETTINGS}
          >
            <img src={settingsIcon} alt={ALTS.SETTINGS} />
          </button>
        </div>
      </header>

      {/* ── Main 3×3 Grid ── */}
      <main className="app-main">
        <div className="launcher-grid">

          {/* Row 1 — PokéCenter */}
          {(['1', '2', '3'] as const).map((v) => (
            <ImageButton
              key={`pc-${v}`}
              defaultSrc={pcDefault}
              hoverSrc={pcHover}
              pressedSrc={pcPressed}
              alt={`PTD ${v} PokéCenter`}
              label={`PTD ${v}\nPokéCenter`}
              onClick={() => handlePokecenter(v)}
            />
          ))}

          {/* Row 2 — Play */}
          {(['PTD1', 'PTD2', 'PTD3'] as const).map((id) => {
            const num = id.replace('PTD', '');
            const s = btnStates[id];
            return (
              <ImageButton
                key={id}
                defaultSrc={playDefault}
                hoverSrc={playHover}
                pressedSrc={playPressed}
                alt={`Play Pokemon TD ${num}`}
                label={`Play\nPokemon TD ${num}`}
                onClick={() => handlePlay(id, `Pokemon TD ${num}`)}
                loading={s.loading}
                progress={s.progress}
                disabled={s.downloaded === null}
              />
            );
          })}

          {/* Row 3 — Hacked */}
          {(['PTD1_Hacked', 'PTD2_Hacked', 'PTD3_Hacked'] as const).map((id) => {
            const num = id.replace('PTD', '').replace('_Hacked', '');
            const s = btnStates[id];
            return (
              <ImageButton
                key={id}
                defaultSrc={hackedDefault}
                hoverSrc={hackedHover}
                pressedSrc={hackedPressed}
                alt={`PTD ${num} Hacked`}
                label={`PTD ${num}\nHacked`}
                onClick={() => handlePlay(id, `PTD ${num} Hacked`)}
                loading={s.loading}
                progress={s.progress}
                disabled={s.downloaded === null}
                textColor="black"
              />
            );
          })}

        </div>
      </main>

      {/* ── Status Bar ── */}
      <footer className="app-footer">
        <div className="status-bar">
          <span className="status-text">{statusMessage}</span>
        </div>
      </footer>

      <SettingsModal
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onStatusChange={setStatusMessage}
        onSoundToggle={(_enabled) => { /* useSounds reads settings fresh on each call */ }}
      />

      {/* ── Update Modal ── */}
      {updateModalOpen && (
        <div className="update-overlay" onClick={(e) => { if (e.currentTarget === e.target) setUpdateModalOpen(false); }}>
          <div className="update-modal">
            <div className="update-modal-header">
              <span>Updates Available</span>
              <button className="update-close" onClick={() => setUpdateModalOpen(false)}>×</button>
            </div>
            <div className="update-modal-body">
              <button
                className="update-all-btn"
                onClick={handleUpdateAll}
                disabled={updatingIds.size > 0}
              >
                {updatingIds.size > 0 ? 'Updating...' : 'Update All'}
              </button>
              <div className="update-game-list">
                {(['PTD1', 'PTD1_Hacked', 'PTD2', 'PTD2_Hacked', 'PTD3', 'PTD3_Hacked'] as GameId[])
                  .filter((id) => updateList.includes(id))
                  .map((id) => {
                    const label = id.replace('_Hacked', ' Hacked').replace('PTD', 'PTD ');
                    const busy = updatingIds.has(id);
                    const prog = btnStates[id]?.progress;
                    return (
                      <div key={id} className="update-game-row">
                        <span className="update-game-name">{label}</span>
                        {busy && prog !== null && (
                          <div className="update-progress-bar">
                            <div style={{ width: `${prog}%` }} />
                          </div>
                        )}
                        <button
                          className="update-single-btn"
                          onClick={() => handleUpdateGame(id)}
                          disabled={busy}
                        >
                          {busy ? (prog !== null ? `${prog}%` : '...') : 'Update'}
                        </button>
                      </div>
                    );
                  })}
              </div>
            </div>
            <div className="update-modal-footer">
              <button className="update-ok" onClick={() => setUpdateModalOpen(false)}>Close</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
