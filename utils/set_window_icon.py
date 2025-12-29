import os
import platform
import tkinter as tk

# function to set the window favicon and prevent malfunction on linux systems
def set_window_icon(window, icon_path):
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