#!/usr/bin/env python3
import argparse
import datetime
import fcntl
import gzip
import hashlib
import http.server
import json
import os
from pathlib import Path
import re
import shutil
import socketserver
import subprocess
import threading
import urllib.error
import urllib.request
import uuid

ACTIVE = {'preparing', 'backing_up', 'installing', 'verifying'}
FIELDS = ('id', 'version', 'phase', 'actor', 'started_at', 'finished_at', 'backup_id', 'message_code')
REPOSITORY = 'Viniciusadm/aphel'
IMAGE = 'ghcr.io/viniciusadm/aphel'


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def version(value):
    match = re.fullmatch(r'v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', value)
    if not match:
        raise ValueError('invalid_version')
    return tuple(map(int, match.groups()))


def atomic_json(path, data):
    temporary = path.with_suffix('.tmp')
    with temporary.open('w') as output:
        os.chmod(temporary, 0o600)
        json.dump(data, output)
        output.flush()
        os.fsync(output.fileno())
    temporary.replace(path)


class Updater:
    def __init__(self, config):
        self.config = config
        self.project = Path(config['project_dir']).resolve(strict=True)
        self.directory = Path(config['state_dir'])
        self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.lock_file = (self.directory / 'service.lock').open('w')
        fcntl.flock(self.lock_file, fcntl.LOCK_EX | fcntl.LOCK_NB)
        self.mutex = threading.RLock()
        self.file = self.directory / 'state.json'
        self.state = json.loads(self.file.read_text()) if self.file.exists() else {
            'checked_at': None, 'latest': None, 'operation': None, 'history': [],
        }
        self.override = self.project / 'compose.updater-image.yml'
        self.docker = ['docker', '--config', config.get('docker_config_dir', '/etc/aphel/registry')]
        self.base = self.docker + ['compose', '--project-directory', str(self.project)]
        for filename in config['compose_files']:
            path = (self.project / filename).resolve(strict=True)
            self.base += ['-f', str(path)]
        self.command = self.base + (['-f', str(self.override)] if self.override.exists() else [])
        operation = self.state.get('operation')
        if operation and operation['phase'] in ACTIVE:
            operation.update(phase='recovery_required', message_code='updater_interrupted', finished_at=now())
            self.state['history'].append(dict(operation))
            self.state['history'] = self.state['history'][-20:]
        elif not operation or operation['phase'] != 'recovery_required':
            self.state['current_version'] = self.installed_version()
        self.save()

    def run(self, arguments, timeout=300, output=None):
        result = subprocess.run(arguments, cwd=self.project, stdout=output or subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=timeout, check=True)
        return result.stdout.decode().strip() if output is None else None

    def compose(self, *arguments, timeout=300, output=None):
        return self.run(self.command + list(arguments), timeout, output)

    def installed_version(self):
        output = self.compose('exec', '-T', 'app', '/usr/local/bin/aphel', '--version', timeout=20)
        installed = output.split()[-1]
        version(installed)
        return installed

    def save(self):
        atomic_json(self.file, self.state)

    def status(self):
        with self.mutex:
            def public(operation):
                return {key: operation.get(key) for key in FIELDS} if operation else None
            return {
                'enabled': True, 'managed_by_console': False,
                'current_version': self.state['current_version'],
                'checked_at': self.state['checked_at'], 'latest': self.state['latest'],
                'operation': public(self.state['operation']),
                'history': [public(operation) for operation in self.state['history'][-20:]][::-1],
            }

    def check(self):
        with self.mutex:
            operation = self.state.get('operation')
            if operation and operation['phase'] in ACTIVE:
                raise ValueError('conflict')
            request = urllib.request.Request(
                f'https://api.github.com/repos/{REPOSITORY}/releases/latest',
                headers={'Accept': 'application/vnd.github+json', 'User-Agent': 'Aphel-Updater'},
            )
            try:
                with urllib.request.urlopen(request, timeout=12) as response:
                    release = json.load(response)
                tag = release['tag_name']
                version(tag)
                if release.get('draft') or release.get('prerelease'):
                    raise ValueError('invalid_release')
                self.state['latest'] = {
                    'version': tag.removeprefix('v'),
                    'url': f'https://github.com/{REPOSITORY}/releases/tag/{tag}',
                    'notes': (release.get('body') or '')[:64000],
                }
            except urllib.error.HTTPError as error:
                if error.code != 404:
                    raise
                self.state['latest'] = None
            if not operation or operation['phase'] != 'recovery_required':
                self.state['current_version'] = self.installed_version()
            self.state['checked_at'] = now()
            self.save()
            return self.status()

    def refresh_loop(self):
        while True:
            try:
                self.check()
            except Exception:
                pass
            threading.Event().wait(21600)

    def start(self, action, actor, operation_id):
        with self.mutex:
            operation = self.state.get('operation')
            if operation and operation['phase'] in ACTIVE | {'recovery_required'}:
                raise ValueError('conflict')
            if action == 'prepare':
                latest = self.state['latest']
                if not latest or version(latest['version']) <= version(self.state['current_version']):
                    raise ValueError('conflict')
                if operation and operation['phase'] == 'prepared' and operation['version'] == latest['version']:
                    return self.status()
                operation = {
                    'id': str(uuid.uuid4()), 'version': latest['version'], 'phase': 'preparing',
                    'actor': actor, 'started_at': now(), 'finished_at': None,
                    'backup_id': None, 'message_code': None,
                }
                self.state['operation'] = operation
            elif action == 'apply':
                if not operation or operation['id'] != operation_id or operation['phase'] != 'prepared':
                    raise ValueError('conflict')
                operation.update(phase='backing_up', actor=actor)
            else:
                raise ValueError('invalid_action')
            self.save()
            worker = threading.Thread(target=self.execute, args=(action,), daemon=False)
            worker.start()
            return self.status()

    def phase(self, phase, **fields):
        with self.mutex:
            self.state['operation'].update(phase=phase, **fields)
            self.save()

    def finish(self, phase, message_code=None):
        with self.mutex:
            self.phase(phase, message_code=message_code, finished_at=now())
            self.state['history'].append(dict(self.state['operation']))
            self.state['history'] = self.state['history'][-20:]
            self.save()

    def ensure_space(self):
        minimum = self.config.get('min_free_disk_mb', 2048) * 1024 * 1024
        storage = json.loads(self.compose('config', '--format', 'json'))
        volumes = storage['services']['app'].get('volumes', [])
        for mount in volumes:
            if mount['target'] == '/app/storage' and mount['type'] != 'volume':
                raise ValueError('unsupported_storage_mount')
        required = minimum
        required += 2 * int(self.compose('exec', '-T', '--user', '0', 'mysql', 'du', '-sb', '/var/lib/mysql').split()[0])
        required += int(self.compose('exec', '-T', '--user', '0', 'app', 'du', '-sb', '/app/storage').split()[0])
        if shutil.disk_usage(self.directory).free < required:
            raise ValueError('insufficient_disk')

    def execute(self, action):
        switched = False
        stopped = False
        try:
            self.ensure_space()
            operation = self.state['operation']
            if action == 'prepare':
                image = f"{IMAGE}:{operation['version']}"
                self.run(self.docker + ['pull', image], timeout=1800)
                info = json.loads(self.run(self.docker + ['image', 'inspect', image]))[0]
                source = (info['Config'].get('Labels') or {}).get('org.opencontainers.image.source')
                if source != f'https://github.com/{REPOSITORY}':
                    raise ValueError('invalid_image')
                installed = self.run(self.docker + ['run', '--rm', '--network', 'none', '--read-only',
                                      '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
                                      image, '--version'], timeout=30).split()[-1]
                if version(installed) != version(operation['version']):
                    raise ValueError('invalid_image')
                digest = next(value for value in info['RepoDigests'] if value.startswith(IMAGE + '@sha256:'))
                self.phase('prepared', image=digest)
                return
            self.backup(operation)
            stopped = True
            self.phase('installing')
            image = operation['image']
            candidate = self.override.with_suffix('.tmp')
            candidate.write_text(f'services:\n  app:\n    image: {image}\n  worker:\n    image: {image}\n')
            candidate.replace(self.override)
            self.command = self.base + ['-f', str(self.override)]
            switched = True
            self.compose('run', '--rm', '--no-deps', '-T', 'app', 'migrate', timeout=1800)
            self.compose('up', '-d', '--no-build', '--wait', '--wait-timeout', '240', 'app', 'worker', timeout=300)
            self.phase('verifying')
            current = self.installed_version()
            if version(current) != version(operation['version']):
                raise ValueError('version_mismatch')
            config = json.loads(self.compose('config', '--format', 'json'))
            environment = config['services']['app'].get('environment', {})
            if environment.get('CONTROL_DATABASE_URL'):
                self.compose('exec', '-T', '-e', 'RUST_LOG=error', 'app', '/usr/local/bin/aphel', 'tenancy-check', timeout=300)
            else:
                self.compose('exec', '-T', 'app', '/usr/local/bin/aphel', 'healthcheck', '--ready', timeout=30)
            with self.mutex:
                self.state['current_version'] = current
            self.finish('succeeded')
        except Exception as error:
            print(f'Update {action} failed: {type(error).__name__}', flush=True)
            if switched:
                try:
                    self.compose('stop', 'app', 'worker', timeout=90)
                except Exception:
                    pass
                self.finish('recovery_required', 'restore_backup_required')
            else:
                if stopped or action == 'apply':
                    try:
                        self.compose('up', '-d', '--no-build', 'app', 'worker', timeout=90)
                    except Exception:
                        self.finish('recovery_required', 'restart_failed')
                        return
                code = str(error) if isinstance(error, ValueError) else 'preparation_failed' if action == 'prepare' else 'backup_failed'
                self.finish('failed', code)

    def backup(self, operation):
        target = self.directory / 'backups' / operation['id']
        target.mkdir(parents=True, mode=0o700)
        self.phase('backing_up', backup_id=operation['id'])
        previous = self.override.read_text() if self.override.exists() else None
        atomic_json(target / 'previous-image.json', {'override': previous})
        self.compose('stop', 'app', 'worker', timeout=90)
        databases = self.compose('exec', '-T', 'mysql', 'sh', '-ec',
                                 'MYSQL_PWD="$MYSQL_ROOT_PASSWORD" exec mysql -uroot -N -e "SHOW DATABASES"').splitlines()
        databases = [name for name in databases if name not in {'mysql', 'information_schema', 'performance_schema', 'sys'}]
        if not databases or any(not re.fullmatch(r'[a-zA-Z0-9_][a-zA-Z0-9_-]*', name) for name in databases):
            raise ValueError('invalid_databases')
        dump = ['exec', '-T', 'mysql', 'sh', '-ec',
                'MYSQL_PWD="$MYSQL_ROOT_PASSWORD" exec mysqldump -uroot --single-transaction --routines --events --triggers --set-gtid-purged=OFF --no-tablespaces --add-drop-database --databases "$@"',
                'backup', *databases]
        with (target / 'database.sql').open('wb') as output:
            self.compose(*dump, timeout=1800, output=output)
        with (target / 'database.sql').open('rb') as source, gzip.open(target / 'database.sql.gz', 'wb') as output:
            shutil.copyfileobj(source, output)
        (target / 'database.sql').unlink()
        with (target / 'storage.tar.gz').open('wb') as output:
            self.compose('run', '--rm', '--no-deps', '-T', '--entrypoint', 'tar', 'app',
                         '-C', '/app/storage', '-czf', '-', '.', timeout=1800, output=output)
        shutil.copy2(self.project / '.env', target / 'installation.env')
        for index, filename in enumerate(self.config['compose_files']):
            shutil.copy2(self.project / filename, target / f'compose-{index}.yml')
        hashes = {}
        for path in target.iterdir():
            if path.is_file():
                with path.open('rb') as source:
                    hashes[path.name] = hashlib.file_digest(source, 'sha256').hexdigest()
                os.chmod(path, 0o600)
        atomic_json(target / 'manifest.json', {'created_at': now(), 'databases': databases, 'sha256': hashes})


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.respond('status' if self.path == '/status' else None)

    def do_POST(self):
        self.respond(self.path.removeprefix('/') if self.path in {'/check', '/prepare', '/apply'} else None)

    def respond(self, action):
        try:
            if action is None:
                self.send_error(404)
                return
            body = {}
            if self.command == 'POST':
                size = int(self.headers.get('Content-Length', '0'))
                if size < 0 or size > 4096:
                    self.send_error(413)
                    return
                body = json.loads(self.rfile.read(size))
                if not isinstance(body.get('actor'), str) or not 1 <= len(body['actor']) <= 255:
                    self.send_error(400)
                    return
            if action == 'status':
                result = self.server.updater.status()
            elif action == 'check':
                result = self.server.updater.check()
            else:
                result = self.server.updater.start(action, body['actor'], body.get('operation_id'))
            self.send_json(200, result)
        except ValueError:
            self.send_json(409, {'error': 'state_conflict'})
        except Exception as error:
            print(f'Update request failed: {type(error).__name__}', flush=True)
            self.send_json(503, {'error': 'updater_unavailable'})

    def send_json(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True)
    args = parser.parse_args()
    config = json.loads(Path(args.config).read_text())
    updater = Updater(config)
    socket = Path(config['socket'])
    socket.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
    os.chmod(socket.parent, 0o755)
    socket.unlink(missing_ok=True)
    with Server(str(socket), Handler) as server:
        os.chown(socket, 0, config.get('socket_gid', 10001))
        os.chmod(socket, 0o660)
        server.updater = updater
        threading.Thread(target=updater.refresh_loop, daemon=True).start()
        print('Aphel updater listening on its local socket.', flush=True)
        server.serve_forever()


if __name__ == '__main__':
    main()
