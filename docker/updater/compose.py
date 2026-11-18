#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

config = json.loads(Path('/etc/aphel/updater.json').read_text())
project = Path(config['project_dir'])
command = ['docker', '--config', config.get('docker_config_dir', '/etc/aphel/registry'),
           'compose', '--project-directory', str(project)]
for filename in config['compose_files']:
    command += ['-f', str(project / filename)]
override = project / 'compose.updater-image.yml'
if override.exists():
    command += ['-f', str(override)]
os.execvp(command[0], command + sys.argv[1:])
