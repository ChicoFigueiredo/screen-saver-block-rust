# Screen Saver Blocker — Rust

> **Mantém o Windows ativo.** Impede economia de energia do monitor, suspensão do sistema e bloqueio de logoff/desligamento — com interface TUI interativa no terminal.

---

## Índice

- [Download rápido](#download-rápido)
- [Visão geral](#visão-geral)
- [Interface TUI](#interface-tui)
- [Compilar localmente](#compilar-localmente)
- [CI/CD e releases automáticas](#cicd-e-releases-automáticas)
- [Assinatura de código (code signing)](#assinatura-de-código-code-signing)
- [Estrutura do projeto](#estrutura-do-projeto)
- [English summary](#english-summary)

---

## Download rápido

Acesse a aba [**Releases**](../../releases/latest) e baixe o arquivo:

```
screen-saver-blocker-windows-x64.zip
```

Extraia o `.exe` e execute diretamente — sem instalação.

> O executável é compilado automaticamente pelo GitHub Actions a cada merge na branch `main`.

---

## Visão geral

Ferramenta de linha de comando para Windows, escrita em Rust, que usa APIs Win32 (via `windows-rs`) para:

| Função | Status inicial |
|--------|----------------|
| Impedir suspensão do sistema | Ativado sempre ao abrir |
| Impedir desligamento do monitor | Alternável via tecla **M** |
| Bloquear desligamento / logoff | Alternável via tecla **S** |

O estado de cada função é exibido em tempo real na interface TUI e pode ser alternado interativamente sem reiniciar o programa.

---

## Interface TUI

A interface é renderizada com [ratatui](https://github.com/ratatui-org/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm) e se adapta ao tamanho do terminal (3 variantes de título responsivo).

```
╔══════════════════════════════════════════════════╗
║   SCREEN SAVER BLOCKER  (título ASCII art)       ║
╠══════════════════════════════════════════════════╣
║                                                  ║
║  [ MONITOR SLEEP ]    Status: BLOQUEADO          ║
║  [ SHUTDOWN/LOGOFF ]  Status: LIBERADO           ║
║                                                  ║
║  M=Monitor  S=Shutdown  Q/ESC=Sair               ║
╚══════════════════════════════════════════════════╝
```

### Atalhos de teclado

| Tecla | Ação |
|-------|------|
| `M` / `m` | Liga/desliga bloqueio do monitor |
| `S` / `s` | Liga/desliga bloqueio de desligamento/logoff |
| `Tab` / `↓` / `↑` | Navega entre os itens |
| `Enter` / `Espaço` | Alterna o item selecionado |
| `Q` / `q` / `ESC` | Encerra o programa |

### Mouse

Clique direto nas linhas dos botões na TUI para alternar os estados.

---

## Compilar localmente

### Requisitos

- **Rust stable** — instale em [rustup.rs](https://rustup.rs)
- **Windows** (obrigatório — usa APIs Win32)
- Toolchain **GNU**: MinGW (`dlltool.exe`) no `PATH` se você quiser compilar nesse ambiente
- Toolchain **MSVC**: Visual Studio Build Tools *(necessário para ícone no .exe)*

### Comandos

```bash
# Verificar dependências
cargo check

# Compilar debug
cargo build

# Compilar release (otimizado)
cargo build --release

# Executar diretamente
cargo run
```

O executável de release fica em `target/release/screen-saver-blocker-rust.exe`.

Se `cargo build --release` falhar no Windows com erro de linkagem como `unable to find library -lgcc_eh` ou `-lgcc`, use o script [build-msvc.cmd](build-msvc.cmd). Ele força o toolchain MSVC, que é o caminho mais estável para este projeto no Windows.

### Ícone no executável

O arquivo `build.rs` embute automaticamente um ícone no `.exe` quando a compilação usa toolchain **MSVC** (`x86_64-pc-windows-msvc`). Com toolchain GNU, o build continua normalmente mas sem ícone — uma mensagem de aviso é exibida no log de compilação.

O GitHub Actions usa `windows-latest` com MSVC, então o binário da release **sempre tem ícone embutido**.

---

## CI/CD e releases automáticas

### Fluxo completo

```
Abrir PR ──► build de verificação (Windows MSVC)
                 │
Merge PR  ──► tag semver auto-incrementada (vX.Y.Z)
                 │
             Compilar release binary (MSVC + ícone embutido)
                 │
             Assinar .exe  ◄── opcional, pula se sem certificado
                 │
             Zipar ──► screen-saver-blocker-windows-x64.zip
                 │
             Publicar GitHub Release com o .zip
```

### Trigger

O workflow `.github/workflows/release-windows.yml` é acionado apenas em pull requests para `main`:

| Evento | O que acontece |
|--------|----------------|
| PR aberto / atualizado | Build de verificação no runner Windows |
| PR mergeado | Tag semver + build + release publicada |

### Versionamento automático

A tag é calculada automaticamente:
- Lê todas as tags `vX.Y.Z` existentes no repositório.
- Incrementa o `patch` (ex.: `v0.1.2` → `v0.1.3`).
- Evita colisão com tags já existentes no remote.

---

## Assinatura de código (code signing)

O workflow suporta assinatura opcional do `.exe`. Sem certificado configurado, a etapa é pulada automaticamente e o binário é distribuído sem assinatura (Windows pode exibir aviso SmartScreen).

### Posso usar Certbot para isso?

Não. O **Certbot** emite certificados TLS/HTTPS (ex.: Let's Encrypt) para sites e servidores, e **não** certificados de assinatura de código.

Para assinar executáveis Windows você precisa de um certificado **Code Signing** em formato PFX (OV/EV) emitido por uma CA compatível, ou usar um certificado autoassinado para testes internos.

### Opção 1 — Certificado auto-assinado (gratuito, para uso pessoal/interno)

Execute o PowerShell abaixo **como Administrador** na sua máquina Windows:

```powershell
# 1. Gerar o certificado no repositório pessoal do usuário
$cert = New-SelfSignedCertificate `
  -Subject "CN=Screen Saver Blocker, O=SeuNome" `
  -Type CodeSigningCert `
  -CertStoreLocation Cert:\CurrentUser\My `
  -NotAfter (Get-Date).AddYears(5)

# 2. Exportar como PFX protegido por senha
$pwd = ConvertTo-SecureString -String "SUA_SENHA_FORTE" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "codesign.pfx" -Password $pwd

# 3. Converter para Base64 e copiar para a área de transferência
[Convert]::ToBase64String([IO.File]::ReadAllBytes("codesign.pfx")) | Set-Clipboard
Write-Host "Base64 copiado. Cole no secret do GitHub."

# 4. Remover o arquivo do disco após copiar
Remove-Item "codesign.pfx" -Force
```

> **Limitação**: certificado auto-assinado **não elimina** o aviso SmartScreen do Windows. O sistema confia apenas em certificados emitidos por autoridades reconhecidas pela Microsoft.

---

### Opção 2 — Certificado comercial (recomendado para distribuição pública)

| Fornecedor | Tipo | Aprox. preço/ano | Remove SmartScreen |
|------------|------|------------------|--------------------|
| [Sectigo](https://sectigo.com/ssl-certificates-tls/code-signing) | OV | ~US$ 200 | Parcial (acumula reputação) |
| [DigiCert](https://www.digicert.com/signing/code-signing-certificates) | OV/EV | ~US$ 300–500 | EV = imediato |
| [GlobalSign](https://www.globalsign.com/en/code-signing-certificate/) | OV/EV | similar | EV = imediato |

- **OV** (Organization Validation): elimina SmartScreen gradualmente conforme o binário ganha reputação.
- **EV** (Extended Validation): requer token físico USB/HSM; elimina SmartScreen imediatamente desde o primeiro download.

Resumo rápido de compra/emissão:
1. Escolha uma CA (Sectigo, DigiCert, GlobalSign etc.) e compre Code Signing OV ou EV.
2. Conclua a validação da organização solicitada pela CA.
3. Receba o certificado (arquivo PFX ou token/HSM, dependendo do plano).
4. Exporte/obtenha um PFX utilizável no CI, converta para Base64 e configure os secrets do GitHub.

---

### Opção 3 — SignPath.io (gratuito para projetos open source)

O [SignPath Foundation](https://signpath.io/product/open-source/) oferece assinatura gratuita para projetos públicos no GitHub. Requer inscrição e aprovação do projeto.

---

### Configurar os secrets no GitHub

Após obter o certificado (qualquer opção), configure no repositório em **Settings → Secrets and variables → Actions**:

| Tipo | Nome | Valor |
|------|------|-------|
| **Secret** | `WINDOWS_CERT_PFX_BASE64` | Conteúdo do `.pfx` em Base64 |
| **Secret** | `WINDOWS_CERT_PASSWORD` | Senha do `.pfx` |
| **Variable** | `WINDOWS_TIMESTAMP_URL` | URL do servidor de timestamp *(opcional)* |

Servidores de timestamp gratuitos (use um compatível com seu certificado):

| Fornecedor | URL |
|------------|-----|
| DigiCert *(padrão do workflow)* | `http://timestamp.digicert.com` |
| Sectigo | `http://timestamp.sectigo.com` |
| GlobalSign | `http://timestamp.globalsign.com/scripts/timstamp.dll` |

> Se `WINDOWS_CERT_PFX_BASE64` não estiver configurado, o workflow **pula a assinatura silenciosamente** e publica o `.zip` sem assinar.

---

## Estrutura do projeto

```
screen.saver.blocker-rust/
├── src/
│   └── main.rs                       # Loop TUI, integração Win32, lógica de bloqueio
├── build.rs                          # Embute ícone no .exe (MSVC only)
├── Cargo.toml                        # Dependências e metadados do executável
├── .github/
│   └── workflows/
│       └── release-windows.yml       # Pipeline CI/CD completo
└── README.md
```

### Dependências principais

| Crate | Versão | Uso |
|-------|--------|-----|
| `ratatui` | 0.28 | Renderização da TUI |
| `crossterm` | 0.29 | Eventos de teclado/mouse, modo raw |
| `windows` | 0.58 | APIs Win32: Power, Shutdown, Messaging |
| `winres` | 0.1 | Embute ícone e metadados no `.exe` *(build-dep)* |

---

## English summary

**Screen Saver Blocker** is a Windows TUI application written in Rust that prevents the system from going to sleep, optionally keeps the monitor on, and optionally blocks shutdown/logoff events — all controllable in real time via keyboard or mouse.

### Quick start

1. Download `screen-saver-blocker-windows-x64.zip` from [**Releases**](../../releases/latest).
2. Extract and run `screen-saver-blocker-windows-x64.exe`.
3. No installation required.

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `M` | Toggle monitor sleep prevention |
| `S` | Toggle shutdown/logoff blocking |
| `Q` / `ESC` | Quit |

### Build from source

```bash
cargo build --release
# Output: target/release/screen-saver-blocker-rust.exe
```

Requires Rust stable on Windows. Icon embedding requires MSVC toolchain.

### Automated releases

Releases are created automatically via GitHub Actions on every PR merged to `main`. The binary is compiled with MSVC (icon embedded), optionally code-signed, zipped, and published as a GitHub Release.

### Code signing

Optional. Configure repository secrets `WINDOWS_CERT_PFX_BASE64` and `WINDOWS_CERT_PASSWORD` with a PFX certificate (Base64-encoded). Without them, the workflow skips signing and publishes unsigned. See the [Assinatura de código](#assinatura-de-código-code-signing) section for full setup instructions.
