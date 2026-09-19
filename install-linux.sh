#!/usr/bin/env bash
# Instala o binário e o lançador para o usuário atual em distribuições Linux.
# Uso: ./install-linux.sh [--uninstall]

set -euo pipefail

APP_ID="io.github.ChicoFigueiredo.BlockScreenSaver"
APP_NAME="block-screen-saver"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${DATA_HOME}/applications"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "${BIN_DIR}/${APP_NAME}" "${DESKTOP_DIR}/${APP_ID}.desktop"
  command -v update-desktop-database >/dev/null && update-desktop-database "${DESKTOP_DIR}" || true
  echo "Block Screen Saver removido da instalação do usuário."
  exit 0
fi

cargo build --release
install -Dm755 "target/release/${APP_NAME}" "${BIN_DIR}/${APP_NAME}"
install -Dm644 "assets/${APP_ID}.desktop" "${DESKTOP_DIR}/${APP_ID}.desktop"
command -v update-desktop-database >/dev/null && update-desktop-database "${DESKTOP_DIR}" || true

echo "Instalado para o usuário atual. Abra 'Block Screen Saver' pelo menu do KDE e use 'Fixar no gerenciador de tarefas'."
