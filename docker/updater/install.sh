#!/usr/bin/env bash
set -euo pipefail

if [[ "$EUID" -ne 0 || "$#" -lt 2 ]]; then
  echo 'Usage: sudo install.sh PROJECT_DIR COMPOSE_FILE [COMPOSE_FILE ...]' >&2
  exit 2
fi

source_dir="$(cd "$(dirname "$0")" && pwd)"
project_dir="$(realpath "$1")"
shift
install -d -m 755 /usr/local/lib/aphel-updater /etc/aphel /run/aphel-updater
install -d -m 700 /var/lib/aphel-updater /etc/aphel/registry
install -m 644 "$source_dir/updater.py" /usr/local/lib/aphel-updater/updater.py
install -m 755 "$source_dir/compose.py" /usr/local/bin/aphel-compose
install -m 644 "$source_dir/aphel-updater.service" /etc/systemd/system/aphel-updater.service
python3 - "$project_dir" "$@" <<'PY'
import json,os,sys
from pathlib import Path
project=Path(sys.argv[1])
files=sys.argv[2:]
for filename in files:
    (project / filename).resolve(strict=True)
path=Path('/etc/aphel/updater.json')
if path.exists():
    raise SystemExit('Updater configuration already exists; review it before reinstalling.')
path.write_text(json.dumps({'project_dir':str(project),'compose_files':files,'state_dir':'/var/lib/aphel-updater','socket':'/run/aphel-updater/updater.sock','socket_gid':10001,'docker_config_dir':'/etc/aphel/registry','min_free_disk_mb':2048},indent=2)+'\n')
os.chmod(path,0o600)
PY
systemctl daemon-reload
systemctl enable --now aphel-updater.service
echo 'Add compose.updater.yml to the installation compose files, then recreate the app service.'
