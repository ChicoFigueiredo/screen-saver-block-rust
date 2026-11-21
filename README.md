# Block Screen Saver

Aplicativo Rust com janela nativa para Windows, Linux e macOS. Ele mantém duas travas enquanto estiver aberto:

- bloqueio de protetor/desligamento automático da tela;
- bloqueio de logoff e de encerramento de sessão.

O aplicativo usa o ícone legado `preferences-desktop-screensaver` como ícone da janela, do executável Windows e do `.app` no macOS.

No Windows, o executável inicia diretamente a interface gráfica, sem abrir uma janela de terminal.

A interface usa `eframe`/`egui` sobre `winit`, portanto não depende de um gerenciador de janelas específico. As integrações ficam isoladas por plataforma:

| Sistema | Tela | Sessão |
| --- | --- | --- |
| Windows | `SetThreadExecutionState` | `ShutdownBlockReasonCreate` |
| Linux | `org.freedesktop.ScreenSaver.Inhibit` | `org.freedesktop.login1.Manager.Inhibit` |
| macOS | `IOPMAssertionCreateWithName` (`PreventUserIdleDisplaySleep`) | `applicationShouldTerminate:` do `NSApplication` |

No Linux, o bloqueio de sessão requer `systemd-logind` e impede desligamento/reinicialização da máquina. O Linux não tem uma API única, independente do ambiente gráfico, para negar exclusivamente o logoff do usuário; GNOME, KDE e outros ambientes expõem APIs próprias para isso. O bloqueio de tela requer que a sessão ofereça a interface D-Bus padronizada de ScreenSaver (GNOME, KDE e vários outros ambientes a oferecem).

No macOS, a asserção de energia impede o protetor de tela, o escurecimento e o repouso da tela por inatividade, e com eles o bloqueio automático da sessão; ela aparece em `pmset -g assertions`. O bloqueio de sessão recusa o pedido de saída que o sistema envia no logout, na reinicialização e no desligamento, e o macOS então cancela a operação. Sair do próprio aplicativo pelo menu (⌘Q) ou pelo Dock continua permitido, e um desligamento forçado (`sudo shutdown`, botão de energia) não passa pelos aplicativos.

No Wayland, o compositor pode derrubar a conexão da janela quando monitores são desligados e religados. No Linux os bloqueios não dependem da janela: se ela cair com erro, o programa se reabre sozinho no mesmo processo, com os bloqueios que estavam ligados, sem soltá-los durante a troca. Após cinco quedas seguidas logo na abertura (como no logoff), ele desiste.

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

O binário final fica em `target/release/block-screen-saver` no Linux e no macOS, e em `target\release\block-screen-saver.exe` no Windows.

### macOS

No macOS, o aplicativo é distribuído como `Block Screen Saver.app` universal (Intel e Apple Silicon, macOS 11 ou mais novo). Para gerá-lo é preciso ter o `rustup` e as Command Line Tools do Xcode (`xcode-select --install`):

```bash
./packaging/macos/build-app.sh
# ou
make bundle-macos
```

O script compila para `x86_64-apple-darwin` e `aarch64-apple-darwin`, junta os dois com `lipo`, monta o bundle com o ícone e o `Info.plist`, assina ad hoc e gera `target/macos/block-screen-saver-macos-universal.zip`.

Para instalar, extraia o `.zip` e mova o aplicativo para `~/Applications` ou `/Applications`. A assinatura é ad hoc, sem notarização: baixado pelo navegador, o macOS bloqueia a primeira abertura. Nesse caso, libere em **Ajustes do Sistema > Privacidade e Segurança > Abrir Mesmo Assim**, ou remova a quarentena:

```bash
xattr -dr com.apple.quarantine ~/Applications/"Block Screen Saver.app"
```

Baixado pelo terminal, o arquivo não recebe quarentena:

```bash
curl -fLO https://github.com/ChicoFigueiredo/screen-saver-block-rust/releases/latest/download/block-screen-saver-macos-universal.zip
ditto -x -k block-screen-saver-macos-universal.zip ~/Applications
open ~/Applications/"Block Screen Saver.app" --args --screen-saver --session
```

### KDE Plasma (Linux)

Quando iniciado diretamente pelo terminal, o Plasma não tem um lançador para associar à janela, então a opção de fixar pode ficar indisponível. Instale o lançador de usuário e abra o aplicativo pelo menu do KDE:

```bash
./install-linux.sh
# ou
make install-linux
```

Depois, clique com o botão direito no ícone do aplicativo no painel e escolha **Fixar no gerenciador de tarefas**. A instalação usa o identificador de janela compatível com Wayland e X11. Para remover: `./install-linux.sh --uninstall`.

## Releases

Todo PR para `main` roda validação de build em Linux e Windows. Quando um PR é mesclado, a pipeline incrementa automaticamente o terceiro dígito da versão, atualiza os manifestos, cria a tag e publica uma GitHub Release com ambos os binários. A pipeline também pode ser executada manualmente para publicar a próxima versão de patch.

O pacote do macOS não é gerado pela pipeline: depois que ela publica a release, ele é compilado em um Mac a partir da tag e anexado à mesma release:

```bash
git fetch --tags && git checkout vX.Y.Z
./packaging/macos/build-app.sh
gh release upload vX.Y.Z target/macos/block-screen-saver-macos-universal.zip
```
