# Screen Saver Blocker (Rust)

A Windows command-line tool written in Rust to keep the computer awake and optionally prevent monitor sleep and system shutdown/logoff prompts.

---

## English

### Overview

This project reproduces the same purpose of the original implementation in `codigos/screen-saver-blocker`, now in Rust.

It uses Windows APIs (via `windows-rs`) to:

- keep the system awake,
- optionally keep the display awake,
- optionally try to block shutdown/logoff while running.

### Features

- `--no-monitor` / `-m`
  - Prevents monitor sleep and system sleep.
- default mode (without `--no-monitor`)
  - Prevents system sleep, allows monitor power saving.
- `--no-kill` / `-k`
  - Installs shutdown/logoff handlers and keeps a hidden message window to respond to end-session events.
- `--help` / `-h`
  - Shows CLI usage.

### Requirements

- Windows
- Rust toolchain
- Recommended: GNU toolchain + MinGW tools available in `PATH`

This repository was validated with:

- Rust stable
- target `x86_64-pc-windows-gnu`
- `dlltool.exe` available (for GNU flow)

### Build

```bash
cargo check
cargo build
```

### Run

```bash
# Show help
cargo run -- --help

# Keep system + monitor awake
cargo run -- --no-monitor

# Keep system awake and enable shutdown/logoff blocking handlers
cargo run -- --no-kill
```

### Behavior Notes

- In non-`--no-kill` mode, press any key to stop.
- In `--no-kill` mode, press `q` to stop.
- On exit, the execution state is reset to `ES_CONTINUOUS`.

### Troubleshooting

If you see linker/tool errors on Windows:

1. Ensure Rust is installed and available in `PATH`.
2. Ensure `dlltool.exe` is available for GNU target builds.
3. If using MSVC target, ensure full Visual C++ build tools are correctly configured.

---

## Portugues (Brasil)

### Visao Geral

Esta ferramenta de linha de comando para Windows foi escrita em Rust para reproduzir a mesma finalidade da versao original em `codigos/screen-saver-blocker`.

Ela usa APIs do Windows (via `windows-rs`) para:

- manter o computador ativo,
- opcionalmente manter o monitor ativo,
- opcionalmente tentar bloquear desligamento/logoff enquanto estiver em execucao.

### Funcionalidades

- `--no-monitor` / `-m`
  - Impede desligamento do monitor e suspensao do sistema.
- modo padrao (sem `--no-monitor`)
  - Impede suspensao do sistema e permite economia de energia do monitor.
- `--no-kill` / `-k`
  - Registra handlers de desligamento/logoff e cria uma janela oculta para responder eventos de fim de sessao.
- `--help` / `-h`
  - Mostra ajuda da linha de comando.

### Requisitos

- Windows
- Rust instalado
- Recomendado: toolchain GNU + ferramentas MinGW no `PATH`

Validado neste projeto com:

- Rust stable
- target `x86_64-pc-windows-gnu`
- `dlltool.exe` disponivel (fluxo GNU)

### Compilacao

```bash
cargo check
cargo build
```

### Execucao

```bash
# Mostrar ajuda
cargo run -- --help

# Manter sistema + monitor ativos
cargo run -- --no-monitor

# Manter sistema ativo com handlers de bloqueio de desligamento/logoff
cargo run -- --no-kill
```

### Observacoes de Comportamento

- No modo sem `--no-kill`, pressione qualquer tecla para encerrar.
- No modo `--no-kill`, pressione `q` para encerrar.
- Ao sair, o estado de execucao eh restaurado para `ES_CONTINUOUS`.

### Solucao de Problemas

Se ocorrer erro de linker/ferramentas no Windows:

1. Verifique se o Rust esta instalado e no `PATH`.
2. Verifique se `dlltool.exe` esta disponivel para builds com target GNU.
3. Se usar target MSVC, confira se as ferramentas de build C++ do Visual Studio estao instaladas e configuradas.
