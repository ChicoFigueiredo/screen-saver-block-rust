#!/usr/bin/env bash
# Compila o aplicativo na plataforma atual.
# Uso: ./build.sh [debug|release|check|test]

set -euo pipefail

case "${1:-release}" in
  release)
    cargo build --release
    echo "Binário gerado em: target/release/block-screen-saver"
    ;;
  debug)
    cargo build
    echo "Binário gerado em: target/debug/block-screen-saver"
    ;;
  check)
    cargo check
    ;;
  test)
    cargo test
    ;;
  *)
    echo "Uso: $0 [debug|release|check|test]" >&2
    exit 2
    ;;
esac
