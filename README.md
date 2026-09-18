# Block Screen Saver

Aplicativo Rust com janela nativa para Windows e Linux. Ele mantém duas travas enquanto estiver aberto:

- bloqueio de protetor/desligamento automático da tela;
- bloqueio de logoff e de encerramento de sessão.

O aplicativo usa o ícone legado `preferences-desktop-screensaver` como ícone da janela e do executável Windows.

A interface usa `eframe`/`egui` sobre `winit`, portanto não depende de um gerenciador de janelas específico. As integrações ficam isoladas por plataforma:

| Sistema | Tela | Sessão |
| --- | --- | --- |
| Windows | `SetThreadExecutionState` | `ShutdownBlockReasonCreate` |
| Linux | `org.freedesktop.ScreenSaver.Inhibit` | `org.freedesktop.login1.Manager.Inhibit` |

No Linux, o bloqueio de sessão requer `systemd-logind` e impede desligamento/reinicialização da máquina. O Linux não tem uma API única, independente do ambiente gráfico, para negar exclusivamente o logoff do usuário; GNOME, KDE e outros ambientes expõem APIs próprias para isso. O bloqueio de tela requer que a sessão ofereça a interface D-Bus padronizada de ScreenSaver (GNOME, KDE e vários outros ambientes a oferecem).

## Executar

```bash
cargo run -- --screen-saver --session
```

As opções são flags: se presentes, iniciam o respectivo bloqueio; se ausentes, ele inicia desligado:

```text
--screen-saver
--session
```

O estado continua podendo ser mudado pelos dois botões da janela.

## Build

É necessário ter a toolchain Rust instalada (`rustup`/`cargo`). Os atalhos compilam para o sistema operacional em que forem executados:

```bash
# Linux/macOS ou Git Bash
./build.sh                 # release
./build.sh debug

# Make
make build
make run RUN_ARGS='--screen-saver --session'
```

No Prompt de Comando do Windows:

```bat
build.bat
build.bat debug
build.bat check
```

O binário final fica em `target/release/block-screen-saver` no Linux e em `target\release\block-screen-saver.exe` no Windows.

## Releases

Ao enviar uma tag no formato `v*`, a pipeline do GitHub Actions compila os binários Linux e Windows e os anexa automaticamente a uma GitHub Release. Por exemplo, a versão atual é publicada com a tag `v0.3.0`.
