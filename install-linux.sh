#!/usr/bin/env bash
# Instala o binário e o lançador para todos os usuários em distribuições Linux.
# Uso: ./install-linux.sh [--uninstall]

set -euo pipefail

readonly APP_ID="io.github.ChicoFigueiredo.BlockScreenSaver"
readonly APP_NAME="block-screen-saver"
readonly PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly BINARY_PATH="${PROJECT_DIR}/target/release/${APP_NAME}"
readonly SYSTEM_BIN_DIR="/usr/local/bin"
readonly SYSTEM_DESKTOP_DIR="/usr/local/share/applications"
readonly SYSTEM_ICON_DIR="/usr/local/share/pixmaps"

if [[ "${EUID}" -eq 0 ]]; then
  SUDO=()
elif command -v sudo >/dev/null; then
  SUDO=(sudo)
else
  echo "A instalação global requer root. Instale o sudo ou execute como root." >&2
  exit 1
fi

refresh_desktop_database() {
  local database_tool
  database_tool="$(command -v update-desktop-database || true)"
  if [[ -n "${database_tool}" ]]; then
    "${SUDO[@]}" "${database_tool}" "${SYSTEM_DESKTOP_DIR}" || true
  fi
}

if [[ "${1:-}" == "--uninstall" ]]; then
  "${SUDO[@]}" rm -f \
    "${SYSTEM_BIN_DIR}/${APP_NAME}" \
    "${SYSTEM_DESKTOP_DIR}/${APP_ID}.desktop" \
    "${SYSTEM_ICON_DIR}/${APP_NAME}.ico"
  refresh_desktop_database
  echo "Block Screen Saver removido da instalação global."
  exit 0
fi

if command -v cargo >/dev/null; then
  cargo build --manifest-path "${PROJECT_DIR}/Cargo.toml" --release
elif [[ ! -x "${BINARY_PATH}" ]]; then
  echo "Cargo não está disponível e não há binário release em ${BINARY_PATH}." >&2
  exit 1
fi

"${SUDO[@]}" install -Dm755 "${BINARY_PATH}" "${SYSTEM_BIN_DIR}/${APP_NAME}"
"${SUDO[@]}" install -Dm644 \
  "${PROJECT_DIR}/assets/${APP_ID}.desktop" \
  "${SYSTEM_DESKTOP_DIR}/${APP_ID}.desktop"
"${SUDO[@]}" install -Dm644 \
  "${PROJECT_DIR}/assets/preferences-desktop-screensaver.ico" \
  "${SYSTEM_ICON_DIR}/${APP_NAME}.ico"
refresh_desktop_database

echo "Instalado globalmente. Abra 'Block Screen Saver' pelo menu do KDE e use 'Fixar no gerenciador de tarefas'."
