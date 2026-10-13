#!/usr/bin/env bash
# Captura la ventana de IdleZero en una pantalla virtual (Xvfb), sin tocar el
# escritorio. Solo para desarrollo.
#
# Uso: tools/screenshot.sh [--wait SEG] [--no-build] [salida.png]
# Requiere: xorg-x11-server-Xvfb e ImageMagick (import).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
wait_s=5
build=1
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --wait) wait_s="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    *) out="$1"; shift ;;
  esac
done
out="${out:-$root/target/screenshots/$(date +%Y%m%d-%H%M%S).png}"
mkdir -p "$(dirname "$out")"

command -v Xvfb >/dev/null || { echo "Falta Xvfb: sudo dnf install xorg-x11-server-Xvfb" >&2; exit 1; }
command -v import >/dev/null || { echo "Falta ImageMagick (import)" >&2; exit 1; }

if [ "$build" = 1 ]; then
  # Binario de depuración con el frontend embebido (no necesita Vite).
  (cd "$root/app" && npm run tauri build -- --no-bundle --debug >/dev/null)
fi
bin="$root/target/debug/idlezero"

display=":99"
while [ -e "/tmp/.X11-unix/X${display#:}" ]; do display=":$(( ${display#:} + 1 ))"; done

pids=()
cleanup() { for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done; }
trap cleanup EXIT

Xvfb "$display" -screen 0 1200x800x24 -nolisten tcp >/dev/null 2>&1 &
pids+=($!)
sleep 1

env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY="$display" "$bin" >/dev/null 2>&1 &
pids+=($!)
sleep "$wait_s"

import -display "$display" -window root "$out"
echo "$out"
