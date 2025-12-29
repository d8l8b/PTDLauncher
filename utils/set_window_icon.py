import os
import platform
import tkinter as tk

# function to set the window favicon and prevent malfunction on linux systems
def set_window_icon(window, base_filename):
    # Determine the extension based on the OS
    if platform.system() == "Windows":
        icon_path = f"{base_filename}.ico"
    else:
        # Using .gif (or .png) for Linux/macOS
        icon_path = f"{base_filename}.gif"
    
    # Check if the file exists after appending the extension
    if not os.path.exists(icon_path):
        print(f"Warning: Icon file not found at {icon_path}")
        return

    try:
        if platform.system() == "Windows":
            window.iconbitmap(icon_path)
        else:
            # using iconphoto for linux according to:
            # https://stackoverflow.com/questions/45361749/python-3-6-tkinter-window-icon-on-linux-error
            # (.ico format only works on windows)
            icon_img = tk.PhotoImage(file=icon_path)
            window.iconphoto(True, icon_img)
    except Exception as e:
        print(f"Failed to set icon for {platform.system()}: {e}")