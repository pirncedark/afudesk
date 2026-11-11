"""Disposable Xvfb receiver: observes actual events, never generates input."""
import json
import os
from pathlib import Path
import sys
import tkinter as tk

output = Path(sys.argv[1])
root = tk.Tk()
root.title("AfuDesk smoke input receiver")
root.geometry("560x200+20+20")
root.configure(background="#335577")
entry = tk.Entry(root, font=("sans", 18))
entry.place(x=20, y=40, width=500, height=50)
state = {"text": "", "enter": False, "clicks": 0}


def save():
    state["text"] = entry.get()
    temp = output / "input.json.tmp"
    temp.write_text(json.dumps(state), encoding="utf-8")
    os.replace(temp, output / "input.json")


def clicked(_):
    state["clicks"] += 1
    root.after_idle(save)


def key(event):
    if event.keysym == "Return":
        state["enter"] = True
    root.after_idle(save)


entry.bind("<ButtonPress-1>", clicked)
entry.bind("<KeyRelease>", key)
root.update()
(output / "input-target.txt").write_text(
    f"{entry.winfo_rootx() + 30} {entry.winfo_rooty() + 20}\n", encoding="ascii"
)
entry.focus_force()
save()
root.mainloop()
