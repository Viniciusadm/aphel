#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Uso: restore.sh [opções] PASTA_DO_BACKUP

Restaura um backup gerado pelo backup.sh. APAGA o banco e os arquivos atuais
do destino e os substitui pelo conteúdo do backup.

Modos:
  --compose ARQUIVO   Instalação Docker: para os serviços app e worker,
                      recria o banco no serviço "mysql" e restaura o volume
                      /app/storage. Os serviços não são religados no fim.
  (sem --compose)     Instalação manual: usa o cliente mysql do host, com a
                      conexão de DATABASE_URL, e o diretório de --storage-dir.
                      Pare o Aphel (serve e worker) antes.

Opções:
  --yes               Não pede confirmação
  --skip-storage      Não restaura os arquivos (arquivos no S3)
  --storage-dir DIR   Diretório local dos arquivos no modo manual (padrão: ./storage)
  -h, --help          Mostra esta ajuda

Variáveis (modo manual):
  DATABASE_URL        mysql://usuario:senha@host:porta/banco (obrigatória)
EOF
}

compose_file=""
assume_yes=false
skip_storage=false
storage_path="./storage"
backup_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --compose) compose_file="${2:?--compose exige um arquivo}"; shift 2 ;;
    --yes) assume_yes=true; shift ;;
    --skip-storage) skip_storage=true; shift ;;
    --storage-dir) storage_path="${2:?--storage-dir exige um diretório}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    -*) echo "Opção desconhecida: $1" >&2; usage >&2; exit 2 ;;
    *) backup_dir="$1"; shift ;;
  esac
done

if [[ -z "$backup_dir" ]]; then
  usage >&2
  exit 2
fi

if [[ ! -f "$backup_dir/database.sql.gz" ]]; then
  echo "database.sql.gz não encontrado em $backup_dir" >&2
  exit 1
fi

if [[ "$skip_storage" == false && ! -f "$backup_dir/storage.tar.gz" ]]; then
  echo "storage.tar.gz não encontrado em $backup_dir (use --skip-storage se os arquivos ficam no S3)" >&2
  exit 1
fi

if [[ -f "$backup_dir/manifest.txt" ]]; then
  echo "Conferindo somas SHA-256"
  (cd "$backup_dir" && grep -E '^[0-9a-f]{64} ' manifest.txt | sha256sum -c -)
fi

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

if [[ -n "$compose_file" ]]; then
  compose=(docker compose -f "$compose_file")
  destination="serviço mysql de $compose_file"
else
  parse_database_url
  destination="$db_host:$db_port/$db_name"
fi

if [[ "$assume_yes" != true ]]; then
  read -r -p "Isto apaga o banco ($destination) e os arquivos atuais. Continuar? [digite sim] " answer
  if [[ "$answer" != "sim" ]]; then
    echo "Cancelado."
    exit 1
  fi
fi

recreate_database_sql() {
  printf 'DROP DATABASE IF EXISTS `%s`;\nCREATE DATABASE `%s` CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;\n' "$1" "$1"
}

if [[ -n "$compose_file" ]]; then
  echo "Parando app e worker"
  "${compose[@]}" stop app worker
  "${compose[@]}" up -d --wait mysql

  db_name="$("${compose[@]}" exec -T mysql printenv MYSQL_DATABASE)"
  echo "Recriando o banco $db_name"
  recreate_database_sql "$db_name" | "${compose[@]}" exec -T mysql sh -c \
    'MYSQL_PWD="$MYSQL_PASSWORD" exec mysql -u"$MYSQL_USER"'

  echo "Importando database.sql.gz"
  gunzip -c "$backup_dir/database.sql.gz" | "${compose[@]}" exec -T mysql sh -c \
    'MYSQL_PWD="$MYSQL_PASSWORD" exec mysql -u"$MYSQL_USER" --default-character-set=utf8mb4 "$MYSQL_DATABASE"'

  if [[ "$skip_storage" == false ]]; then
    echo "Restaurando /app/storage"
    "${compose[@]}" run --rm --no-deps -T --entrypoint sh app -c \
      'find /app/storage -mindepth 1 -delete && tar -C /app/storage -xzf -' \
      < "$backup_dir/storage.tar.gz"
  fi

  echo "Concluído. Suba de novo com: docker compose -f $compose_file up -d"
else
  echo "Recriando o banco $db_name"
  recreate_database_sql "$db_name" \
    | MYSQL_PWD="$db_password" mysql -h "$db_host" -P "$db_port" -u "$db_user"

  echo "Importando database.sql.gz"
  gunzip -c "$backup_dir/database.sql.gz" \
    | MYSQL_PWD="$db_password" mysql -h "$db_host" -P "$db_port" -u "$db_user" \
      --default-character-set=utf8mb4 "$db_name"

  if [[ "$skip_storage" == false ]]; then
    echo "Restaurando $storage_path"
    mkdir -p "$storage_path"
    find "$storage_path" -mindepth 1 -delete
    tar -C "$storage_path" -xzf "$backup_dir/storage.tar.gz"
  fi

  echo "Concluído. Suba o Aphel de novo (aphel serve e aphel worker)."
fi
