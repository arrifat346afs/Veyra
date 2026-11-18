#!/usr/bin/env python3
"""Select Veyra for FileChooser, preserving the effective desktop portal defaults."""
import datetime
import os
from pathlib import Path
import re
import sys

KEY = "org.freedesktop.impl.portal.FileChooser"


def configure(text, enabled=True):
    lines = text.splitlines(keepends=True)
    output = []
    preferred = False
    found = False
    for line in lines:
        if line.strip().startswith("["):
            if preferred and not found:
                output.append(f"{KEY}=veyra\n" if enabled else "")
                found = True
            preferred = line.strip() == "[preferred]"
        if preferred and re.match(r"\s*" + re.escape(KEY) + r"\s*=", line):
            if not found and enabled:
                output.append(f"{KEY}=veyra\n")
            found = True
        else:
            output.append(line if line.endswith("\n") else line + "\n")
    if not found and enabled:
        if not preferred:
            output.append("\n[preferred]\n")
        output.append(f"{KEY}=veyra\n")
    return "".join(output)


def main():
    if sys.argv[1:] not in ([], ["--disable"]):
        sys.exit("Usage: configure-portal.py [--disable]")
    enabled = "--disable" not in sys.argv
    config_home = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config")))
    desktops = [d.lower() for d in os.environ.get("XDG_CURRENT_DESKTOP", "").split(":") if d]
    # A desktop-specific file takes priority over generic portals.conf in the
    # same directory, so update the existing effective user file if present.
    names = [d + "-portals.conf" for d in desktops] + ["portals.conf"]
    user_dir = config_home / "xdg-desktop-portal"
    target = next((user_dir / n for n in names if (user_dir / n).is_file()), user_dir / "portals.conf")
    source = target if target.is_file() else None
    if source is None:
        dirs = os.environ.get("XDG_CONFIG_DIRS", "/etc/xdg").split(":") + ["/etc"]
        dirs += [os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))]
        dirs += os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":")
        source = next((Path(d) / "xdg-desktop-portal" / n for d in dirs for n in names if d and (Path(d) / "xdg-desktop-portal" / n).is_file()), None)
    text = source.read_text() if source else "[preferred]\ndefault=*\n"
    updated = configure(text, enabled)
    if target.is_file() and target.read_text() == updated:
        print(f"Already configured: {target}")
        return
    if target.is_file():
        backup = target.with_name(target.name + ".bak-" + datetime.datetime.now().strftime("%Y%m%d-%H%M%S-%f"))
        backup.write_bytes(target.read_bytes())
        print(f"Backup: {backup}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(updated)
    print(f"{'Enabled' if enabled else 'Disabled'} Veyra FileChooser: {target}")
    print("Apply with: systemctl --user restart xdg-desktop-portal.service")


if __name__ == "__main__":
    main()
