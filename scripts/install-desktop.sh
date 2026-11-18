#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
bin_dir="$HOME/.local/bin"

install -Dm755 "$script_dir/../veyra" "$bin_dir/veyra"
install -Dm644 "$script_dir/../integration/veyra.desktop" \
  "$data_home/applications/veyra.desktop"

for size in 16 24 32 48 64 128 256 512; do
  install -Dm644 "$script_dir/../icons/hicolor/${size}x${size}/apps/veyra.png" \
    "$data_home/icons/hicolor/${size}x${size}/apps/veyra.png"
done

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$data_home/applications" || true
fi

printf 'Installed Veyra. Launch it from your app menu or run: %s/veyra\n' "$bin_dir"
