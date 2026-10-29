#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Uso: backup.sh [opções]

Gera um backup do Aphel: dump do MySQL (database.sql.gz), arquivos enviados
(storage.tar.gz) e um manifest.txt com somas SHA-256.

Modos:
  --compose ARQUIVO   Instalação Docker: roda o mysqldump dentro do serviço
                      "mysql" e lê os arquivos do volume do serviço "app".
  (sem --compose)     Instalação manual: usa o mysqldump do host, com a conexão
                      de DATABASE_URL, e o diretório de --storage-dir.

Opções:
  --output DIR        Diretório onde a pasta do backup é criada (padrão: ./backups)
  --skip-storage      Não copia os arquivos (arquivos no S3)
  --storage-dir DIR   Diretório local dos arquivos no modo manual (padrão: ./storage)
  -h, --help          Mostra esta ajuda

Variáveis (modo manual):
  DATABASE_URL        mysql://usuario:senha@host:porta/banco (obrigatória)
EOF
}

compose_file=""
output_dir="./backups"
skip_storage=false
storage_path="./storage"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --compose) compose_file="${2:?--compose exige um arquivo}"; shift 2 ;;
    --output) output_dir="${2:?--output exige um diretório}"; shift 2 ;;
    --skip-storage) skip_storage=true; shift ;;
    --storage-dir) storage_path="${2:?--storage-dir exige um diretório}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Opção desconhecida: $1" >&2; usage >&2; exit 2 ;;
  esac
done

dump_options=(--single-transaction --routines --triggers --no-tablespaces --default-character-set=utf8mb4)

parse_database_url() {
  local url="${DATABASE_URL:?defina DATABASE_URL (mysql://usuario:senha@host:porta/banco)}"
  local pattern='^mysql://([^:@/]+)(:([^@]*))?@([^:/?]+)(:([0-9]+))?/([^?]+)'
  if [[ ! "$url" =~ $pattern ]]; then
    echo "DATABASE_URL inválida: esperado mysql://usuario:senha@host:porta/banco" >&2
    exit 1
  fi
  db_user="${BASH_REMATCH[1]}"
  db_password="${BASH_REMATCH[3]}"
  db_host="${BASH_REMATCH[4]}"
  db_port="${BASH_REMATCH[6]:-3306}"
  db_name="${BASH_REMATCH[7]}"
}

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
target="${output_dir%/}/aphel-backup-${timestamp}"
mkdir -p "$target"
echo "Backup em $target"

if [[ -n "$compose_file" ]]; then
  compose=(docker compose -f "$compose_file")
  echo "Banco: mysqldump no serviço mysql de $compose_file"
  "${compose[@]}" exec -T mysql sh -c \
    'MYSQL_PWD="$MYSQL_PASSWORD" exec mysqldump -u"$MYSQL_USER" '"${dump_options[*]}"' "$MYSQL_DATABASE"' \
    | gzip > "$target/database.sql.gz"
else
  parse_database_url
  echo "Banco: mysqldump em $db_host:$db_port/$db_name"
  MYSQL_PWD="$db_password" mysqldump -h "$db_host" -P "$db_port" -u "$db_user" \
    "${dump_options[@]}" "$db_name" | gzip > "$target/database.sql.gz"
fi

if ! zcat "$target/database.sql.gz" | tail -n 1 | grep -q "Dump completed"; then
  echo "O dump parece incompleto (sem a linha final 'Dump completed')." >&2
  exit 1
fi

if [[ "$skip_storage" == true ]]; then
  echo "Arquivos: ignorados (--skip-storage)"
elif [[ -n "$compose_file" ]]; then
  echo "Arquivos: volume /app/storage do serviço app"
  "${compose[@]}" run --rm --no-deps -T --entrypoint tar app -C /app/storage -czf - . \
    > "$target/storage.tar.gz"
else
  if [[ ! -d "$storage_path" ]]; then
    echo "Diretório de arquivos não encontrado: $storage_path" >&2
    exit 1
  fi
  echo "Arquivos: $storage_path"
  tar -C "$storage_path" -czf "$target/storage.tar.gz" .
fi

{
  echo "created_at=$timestamp"
  echo "mode=$([[ -n "$compose_file" ]] && echo compose || echo manual)"
  (cd "$target" && sha256sum database.sql.gz $([[ -f storage.tar.gz ]] && echo storage.tar.gz))
} > "$target/manifest.txt"

echo "Concluído:"
ls -lh "$target"
