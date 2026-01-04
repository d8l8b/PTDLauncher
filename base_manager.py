#!/usr/bin/env python3
import platform
import tkinter as tk
from tkinter import Toplevel, Label, Button, Frame, messagebox

class BaseManager:
    def __init__(self, status_callback=None):
        self.status_callback = status_callback
    
    def set_status(self, message):
        """Update status message"""
        if self.status_callback:
            self.status_callback(message)
        else:
            print(message)
    
    def center_window(self, window, parent):
        """Center a window on its parent"""
        window.update_idletasks()
        window.update()
        
        # If parent is None, center on screen
        if parent is None:
            # Get screen dimensions
            screen_width = window.winfo_screenwidth()
            screen_height = window.winfo_screenheight()
            
            # Calculate position to center on screen
            width = window.winfo_width()
            height = window.winfo_height()
            x = (screen_width // 2) - (width // 2)
            y = (screen_height // 2) - (height // 2)
        else:
            # Get parent window position and size
            parent_x = parent.winfo_x()
            parent_y = parent.winfo_y()
            parent_width = parent.winfo_width()
            parent_height = parent.winfo_height()
            
            # Calculate position
            width = window.winfo_width()
            height = window.winfo_height()
            x = parent_x + (parent_width // 2) - (width // 2)
            y = parent_y + (parent_height // 2) - (height // 2)
        
        # Set position
        window.geometry(f"{width}x{height}+{x}+{y}")
    
    def show_dialog(self, parent, title, message, dialog_type="yesno", width=350, height=150):
        """Show a custom dialog centered on parent window
        
        Args:
            parent: Parent window
            title: Dialog title
            message: Dialog message
            dialog_type: Type of dialog (yesno, info, error)
            width: Dialog width
            height: Dialog height
            
        Returns:
            Boolean result for yesno dialogs, None for others
        """
        # If no parent window provided, fallback to standard messagebox
        if not parent:
            if dialog_type == "yesno":
                return messagebox.askyesno(title, message)
            elif dialog_type == "error":
                return messagebox.showerror(title, message)
            elif dialog_type == "info":
                return messagebox.showinfo(title, message)
            return None
        
        # Create a custom dialog
        dialog = Toplevel(parent)
        dialog.title(title)
        dialog.geometry(f"{width}x{height}")
        dialog.minsize(width, height)
        dialog.resizable(False, False)
        dialog.transient(parent)  # Set to be on top of the parent window
        dialog.grab_set()  # Make the dialog modal
        
        # Center the dialog on the parent window
        self.center_window(dialog, parent)

        # Create dialog content with a frame to allow text wrapping
        message_frame = Frame(dialog, padx=20, pady=10)
        message_frame.pack(fill=tk.BOTH, expand=True)
        
        message_label = Label(message_frame, text=message, pady=10, wraplength=width-40)
        message_label.pack(fill=tk.BOTH, expand=True)
        
        # Create buttons
        btn_frame = Frame(dialog)
        btn_frame.pack(pady=10)
        
        result = [False]  # Use a list to store the result
        
        if dialog_type == "yesno":
            def on_yes():
                result[0] = True
                dialog.destroy()
            
            def on_no():
                result[0] = False
                dialog.destroy()
            
            Button(btn_frame, text="Yes", width=10, command=on_yes).pack(side="left", padx=10)
            Button(btn_frame, text="No", width=10, command=on_no).pack(side="left", padx=10)
        else:  # info or error
            def on_ok():
                dialog.destroy()
            
            Button(btn_frame, text="OK", width=10, command=on_ok).pack(side="left", padx=10)
        
        # Wait for the dialog to be closed
        parent.wait_window(dialog)
        
        # Return the result for yesno dialogs
        if dialog_type == "yesno":
            return result[0]
        return None
