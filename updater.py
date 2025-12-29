#!/usr/bin/env python3
import threading
import time
import os
import requests
import tkinter as tk
from base_manager import BaseManager
from config import resource_path
from set_window_icon import set_window_icon

class UpdateManager(BaseManager):
    def __init__(self, config_manager, game_manager, download_manager=None, status_callback=None):
        super().__init__(status_callback)
        self.config_manager = config_manager
        self.game_manager = game_manager
        self.download_manager = download_manager
        self.is_updating = False # To prevent multiple update downloads at once

    def _extract_filename_and_version(self, url, response):
        """Extract filename and version from URL or response headers"""
        filename = ""
        version = ""
        
        if "content-disposition" in response.headers:
            try:
                filename = response.headers['content-disposition'].split('filename=')[1].strip('"')
            except (IndexError, KeyError):
                filename = url.split('/')[-1]
        else:
            filename = url.split('/')[-1]
        
        if '-v' in filename:
            try:
                version = filename.split('-v')[1].split('.swf')[0]
            except (IndexError, KeyError):
                version = str(int(time.time()))
        else:
            version = str(int(time.time()))
            
        return filename, version
    
    def check_updates(self, root=None):
        """Check for updates to Flash Player and games"""
        if self.download_manager and self.download_manager.is_download_in_progress():
            self.show_dialog(root, "Download in Progress", 
                           "Cannot check for updates while another download is in progress.", 
                           dialog_type="info")
            return

        if self.is_updating:
            self.show_dialog(root, "Update in Progress", 
                           "An update process is already running.", 
                           dialog_type="info")
            return

        self.set_status("Checking for updates...")
        thread = threading.Thread(target=self._check_updates_thread, args=(root,))
        thread.daemon = True
        thread.start()
    
    def _check_updates_thread(self, root):
        """Background thread for checking updates"""
        try:
            updates_available = False
            update_messages = []
            
            for game, current_version in self.config_manager.version["games"].items():
                if game in self.config_manager.config["game_urls"]:
                    try:
                        url = self.config_manager.config["game_urls"][game]
                        response = requests.head(url, timeout=10)
                        response.raise_for_status()
                        
                        _, server_version = self._extract_filename_and_version(url, response)
                        
                        if not current_version or current_version != server_version:
                            updates_available = True
                            update_messages.append(f"{game}: v{current_version or 'none'} → v{server_version}")
                    except Exception as e:
                        print(f"Error checking updates for {game}: {str(e)}")
            
            if updates_available and root:
                update_text = "Updates available: " + ", ".join(update_messages)
                root.after(0, lambda: self.set_status(update_text))
                root.after(0, lambda: self._show_update_dialog(root, update_messages))
            else:
                root.after(0, lambda: self.set_status("No updates available"))
                
        except Exception as e:
            root.after(0, lambda: self.set_status(f"Error checking updates: {str(e)}"))
    
    def _show_update_dialog(self, root, update_messages):
        """Show a simple, stateless dialog with available updates"""
        update_window = tk.Toplevel(root)
        update_window.title("Updates Available")
        update_window.geometry("400x320")
        update_window.resizable(False, False)
        update_window.transient(root)
        update_window.grab_set()
        set_window_icon(update_window, resource_path("resources/favicon-original.ico"))

        self.center_window(update_window, root)

        tk.Label(update_window, text="The following updates are available:").pack(pady=10)
        
        updates_frame = tk.Frame(update_window)
        updates_frame.pack(fill=tk.BOTH, expand=True, padx=10, pady=5)
        
        game_rows = {}
        
        for i, message in enumerate(update_messages):
            game = message.split(":")[0]
            
            game_frame = tk.Frame(updates_frame)
            game_frame.grid(row=i, column=0, sticky=tk.W+tk.E, pady=2)
            game_frame.columnconfigure(0, weight=1)
            
            tk.Label(game_frame, text=message, anchor=tk.W).grid(row=0, column=0, sticky=tk.W)
            
            progress_label = tk.Label(game_frame, text="", width=15, anchor=tk.E)
            progress_label.grid(row=0, column=1, sticky=tk.E, padx=5)
            
            download_btn = tk.Button(game_frame, text="Download",
                                   command=lambda g=game: self._download_update(g, game_rows, download_all_btn))
            download_btn.grid(row=0, column=2, padx=5, sticky=tk.E)
            
            game_rows[game] = {
                'progress_label': progress_label,
                'download_btn': download_btn,
                'frame': game_frame,
                'active': True
            }
        
        btn_frame = tk.Frame(update_window)
        btn_frame.pack(fill=tk.X, padx=10, pady=10)
        
        download_all_btn = tk.Button(btn_frame, text="Download All", 
                                   command=lambda: self._download_all_updates(update_messages, game_rows, download_all_btn))
        download_all_btn.pack(side=tk.LEFT, padx=5)
        
        tk.Button(btn_frame, text="Close", command=update_window.destroy).pack(side=tk.RIGHT, padx=5)

    def _toggle_buttons(self, game_rows, download_all_btn, state):
        """Enable or disable all download buttons."""
        try:
            if download_all_btn:
                download_all_btn.config(state=state)
            for game, row in game_rows.items():
                # Only toggle buttons for active rows
                if row.get('active', False):
                    row['download_btn'].config(state=state)
        except (tk.TclError, KeyError):
            # Window or widgets might have been destroyed
            pass

    def _download_update(self, game, game_rows, download_all_btn):
        """Download a single game update."""
        if self.is_updating:
            return
        
        self._toggle_buttons(game_rows, download_all_btn, tk.DISABLED)
        self.is_updating = True

        thread = threading.Thread(target=self._download_worker, args=([game], game_rows, download_all_btn))
        thread.daemon = True
        thread.start()

    def _download_all_updates(self, update_messages, game_rows, download_all_btn):
        """Download all available updates."""
        if self.is_updating:
            return
            
        # Get the list of games that are still active in the UI
        games_to_download = [game for game, row in game_rows.items() if row.get('active', False)]
        
        if not games_to_download:
            self.set_status("All available updates have been downloaded.")
            return

        self._toggle_buttons(game_rows, download_all_btn, tk.DISABLED)
        self.is_updating = True

        thread = threading.Thread(target=self._download_worker, args=(games_to_download, game_rows, download_all_btn))
        thread.daemon = True
        thread.start()

    def _download_worker(self, games, game_rows, download_all_btn):
        """Worker thread to download a list of games sequentially."""
        for game in games:
            try:
                ui_row = game_rows.get(game)
                if not ui_row:
                    continue

                # Define a thread-safe UI update function
                def update_ui(progress, downloaded=None, total=None):
                    try:
                        # Update status bar
                        if progress < 100:
                            status_msg = f"Downloading {game}: {progress}%"
                            if downloaded is not None and total is not None and total > 0:
                                downloaded_mb = downloaded / (1024 * 1024)
                                total_mb = total / (1024 * 1024)
                                status_msg = f"Downloading {game}: {progress}% ({downloaded_mb:.1f}/{total_mb:.1f} MB)"
                            self.set_status(status_msg)
                        else:
                            self.set_status(f"Download complete: {game}")

                        # Update dialog UI
                        if progress < 100:
                            progress_text = f"{progress}%"
                            if downloaded is not None and total is not None and total > 0:
                                downloaded_mb = downloaded / (1024 * 1024)
                                total_mb = total / (1024 * 1024)
                                progress_text = f"{progress}% ({downloaded_mb:.1f}/{total_mb:.1f} MB)"
                            ui_row['progress_label'].config(text=progress_text)
                            ui_row['download_btn'].config(text="Downloading...")
                        else:
                            ui_row['progress_label'].config(text="Done!")
                            ui_row['download_btn'].config(text="Downloaded", state=tk.DISABLED)
                            # Mark as inactive so it's not re-enabled, then schedule for removal
                            ui_row['active'] = False
                            if 'frame' in ui_row:
                                ui_row['frame'].after(500, ui_row['frame'].destroy)
                    except (tk.TclError, KeyError):
                        # Widget was destroyed
                        pass
                
                # Initial UI update
                ui_row['progress_label'].after(0, lambda: update_ui(0))

                # Download the game
                file_path, _ = self._download_game_internal(game, progress_callback=lambda p, d, t: ui_row['progress_label'].after(0, lambda: update_ui(p, d, t)))

                if file_path:
                    # Final UI update for success
                    ui_row['progress_label'].after(0, lambda: update_ui(100))
                else:
                    # UI update for failure
                    try:
                        ui_row['progress_label'].after(0, lambda: ui_row['progress_label'].config(text="Error!"))
                        ui_row['download_btn'].after(0, lambda: ui_row['download_btn'].config(text="Failed"))
                        self.set_status(f"Error downloading {game}")
                    except (tk.TclError, KeyError):
                        pass
                
                time.sleep(0.5) # Small delay between downloads

            except Exception as e:
                print(f"Error in download worker for {game}: {str(e)}")

        # Re-enable buttons when all downloads are done
        self.is_updating = False
        self._toggle_buttons(game_rows, download_all_btn, tk.NORMAL)
        self.set_status("Update process finished.")

    def _download_game_internal(self, game, progress_callback=None, parent=None):
        """Core download functionality."""
        try:
            url = self.config_manager.config["game_urls"][game]
            response = requests.head(url, timeout=10)
            response.raise_for_status()
            
            _, version = self._extract_filename_and_version(url, response)
            
            game_filename = f"{game}.swf"
            file_path = os.path.join(self.config_manager.games_dir, game_filename)
            
            with requests.get(url, stream=True, timeout=30) as r:
                r.raise_for_status()
                total_size = int(r.headers.get('content-length', 0))
                downloaded = 0
                
                with open(file_path, 'wb') as f:
                    for chunk in r.iter_content(chunk_size=8192):
                        if chunk:
                            f.write(chunk)
                            downloaded += len(chunk)
                            if progress_callback and total_size > 0:
                                progress = int((downloaded / total_size) * 100)
                                progress_callback(progress, downloaded, total_size)
                                time.sleep(0.01)
            
            self.config_manager.version["games"][game] = version
            self.config_manager.save_version_info()
            self.set_status(f"{game} v{version} downloaded successfully")
            return file_path, version
            
        except Exception as e:
            error_msg = f"Failed to download {game}: {str(e)}"
            self.set_status(f"Failed to download {game}")
            if parent:
                self.show_dialog(parent, "Error", error_msg, dialog_type="error")
            return None, None

    def download_game(self, game, parent=None):
        """Public method to download a game. Delegates to DownloadManager."""
        if self.download_manager:
            return self.download_manager.download_game(game, parent)
        else:
            # Fallback if no download manager is provided
            return self._download_game_internal(game, parent=parent)[0]
