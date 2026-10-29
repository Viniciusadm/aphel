#!/bin/sh
set -u

RESET_TIME="${DEMO_RESET_TIME:-03:00}"

reset_and_seed() {
  aphel demo reset --full
}

until aphel migrate; do
  echo "demo: database not ready, retrying in 10 seconds"
  sleep 10
done
until aphel demo seed; do
  echo "demo: waiting for the installation (open the app and finish the installer), retrying in 60 seconds"
  sleep 60
done

while true; do
  now=$(date -u +%s)
  target=$(date -u -d "today $RESET_TIME" +%s)
  if [ "$target" -le "$now" ]; then
    target=$(date -u -d "tomorrow $RESET_TIME" +%s)
  fi
  echo "demo: next reset at $(date -u -d "@$target" '+%Y-%m-%d %H:%M') UTC"
  sleep $((target - now))
  if reset_and_seed; then
    echo "demo: reset finished"
  else
    echo "demo: reset failed, retrying in 10 minutes"
    sleep 600
    reset_and_seed || echo "demo: reset failed again"
  fi
done
