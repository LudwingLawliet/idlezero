#!/usr/bin/env bash
# Captura la ventana de IdleZero en una pantalla virtual (Xvfb), sin tocar el
# escritorio. Solo para desarrollo.
#
# Uso: tools/screenshot.sh [--wait SEG] [--no-build] [--click X,Y]... [salida.png]
#   --click X,Y  hace clic en esa posición antes de capturar (requiere xdotool);
#                se puede repetir, en orden.
#   --after SEG  espera tras los clics antes de capturar (por defecto 1).
#   --scroll X,Y,N  gira la rueda N pasos hacia abajo en X,Y (tras los clics).
# Requiere: xorg-x11-server-Xvfb e ImageMagick (import).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
wait_s=5
build=1
clicks=()
scrolls=()
after=1
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --wait) wait_s="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    --click) clicks+=("$2"); shift 2 ;;
    --after) after="$2"; shift 2 ;;
    --scroll) scrolls+=("$2"); shift 2 ;;
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
cleanup() {
  for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
  [ -n "${profile:-}" ] && rm -rf "$profile"
}
trap cleanup EXIT

Xvfb "$display" -screen 0 1200x800x24 -nolisten tcp >/dev/null 2>&1 &
pids+=($!)
sleep 1

# Perfil temporal: las capturas no tocan las preferencias de la app real.
profile="$(mktemp -d)"
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY="$display" XDG_DATA_HOME="$profile/data" XDG_CONFIG_HOME="$profile/config" \
  XDG_CACHE_HOME="$profile/cache" "$bin" >/dev/null 2>&1 &
pids+=($!)
sleep "$wait_s"

for c in "${clicks[@]}"; do
  DISPLAY="$display" xdotool mousemove --sync "${c%,*}" "${c#*,}" click 1
  sleep 1
done
for sc in "${scrolls[@]}"; do
  IFS=, read -r sx sy sn <<<"$sc"
  DISPLAY="$display" xdotool mousemove --sync "$sx" "$sy" click --repeat "$sn" --delay 60 5
  sleep 1
done
sleep "$after"

import -display "$display" -window root "$out"
echo "$out"
