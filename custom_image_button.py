import tkinter as tk

class CustomImageButton(tk.Canvas):
    def __init__(self, parent, text, width, height, 
                 img_default, img_hover, img_pressed, 
                 command=None, text_offset=2,
                 text_color="white", font=("Terminal", 15), bg_color="#F8F8F8"):
        
        super().__init__(parent, width=width, height=height, 
                         bg=bg_color, bd=0, highlightthickness=0, cursor="hand2")
        
        self.command = command
        self.text_offset = text_offset
        self.is_pressed = False
        
        self.default_x = width // 2
        self.default_y = (height // 2) -6
        self.pressed_y = self.default_y + text_offset 
        
        self.img_default = img_default
        self.img_hover = img_hover
        self.img_pressed = img_pressed
        
        self.bg_id = self.create_image(width // 2, height // 2, image=self.img_default)
        self.text_id = self.create_text(self.default_x, self.default_y, text=text, 
                                        fill=text_color, font=font, justify="center", anchor=tk.CENTER)

        self.bind("<Enter>", self.on_enter)
        self.bind("<Leave>", self.on_leave)
        self.bind("<Button-1>", self.on_press)
        self.bind("<ButtonRelease-1>", self.on_release)

    def on_enter(self, event):
        if not self.is_pressed:
            self.itemconfig(self.bg_id, image=self.img_hover)

    def on_leave(self, event):
        self.is_pressed = False
        self.itemconfig(self.bg_id, image=self.img_default)
        self.coords(self.text_id, self.default_x, self.default_y)

    def on_press(self, event):
        self.is_pressed = True
        self.itemconfig(self.bg_id, image=self.img_pressed)
        self.coords(self.text_id, self.default_x, self.pressed_y)

    def on_release(self, event):
        if self.is_pressed:
            self.is_pressed = False
            self.coords(self.text_id, self.default_x, self.default_y)
            
            if 0 <= event.x <= self.winfo_width() and 0 <= event.y <= self.winfo_height():
                self.itemconfig(self.bg_id, image=self.img_hover)
                if self.command:
                    self.command()
            else:
                self.itemconfig(self.bg_id, image=self.img_default)