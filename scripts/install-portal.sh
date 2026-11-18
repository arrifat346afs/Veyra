#!/usr/bin/env bash
# Run cargo build --release as your regular user before installing.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary=${1:-"$project_dir/target/release/veyra"}
prefix=${PREFIX:-/usr/local}
staging=${DESTDIR:-}
portal_dir=${PORTAL_DIR:-/usr/share/xdg-desktop-portal/portals}
if [[ ! -x "$binary" ]]; then
    printf 'Build first with cargo build --release, or pass a compiled binary.\n' >&2
    exit 1
fi
install -Dm755 "$binary" "$staging$prefix/bin/veyra"
install -Dm644 "$project_dir/integration/veyra.portal" "$staging$portal_dir/veyra.portal"
# D-Bus activation files use desktop-entry command-line quoting.
python3 - "$project_dir" "$staging" "$prefix" <<'PY'
import pathlib, sys
project, staging, prefix = sys.argv[1:]
binary = prefix + '/bin/veyra'
quoted = '"' + binary.replace('\\', '\\\\').replace('"', '\\"') + '"'
text = (pathlib.Path(project) / 'integration/org.freedesktop.impl.portal.desktop.veyra.service.in').read_text().replace('@BINARY@', quoted)
target = pathlib.Path(staging + prefix + '/share/dbus-1/services/org.freedesktop.impl.portal.desktop.veyra.service')
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(text)
PY
printf 'Installed Veyra and its FileChooser backend.\nRun scripts/configure-portal.py as your regular user to enable it.\n'
