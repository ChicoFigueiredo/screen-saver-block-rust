//! Reabertura do programa quando a janela cai com erro no Linux.
//!
//! No Wayland o compositor pode derrubar a conexão do cliente por erro de
//! protocolo (por exemplo, `invalid global wl_output` quando um monitor some e
//! volta), e o winit não permite recriar o laço de eventos no mesmo processo.
//! Sem a janela, os inibidores D-Bus iriam embora junto; por isso o processo se
//! re-executa no mesmo PID, mantendo a unidade do systemd e os bloqueios ativos.

use std::{os::unix::process::CommandExt, process::Command, thread, time::Duration};

const ATTEMPT_VAR: &str = "BLOCK_SCREEN_SAVER_RELAUNCH_ATTEMPT";

/// Como reabrir o programa após uma queda da janela.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    attempt: u32,
    delay: Duration,
    args: Vec<&'static str>,
}

/// A partir deste tempo de vida, a queda é tratada como incidente isolado e a
/// contagem de tentativas recomeça.
const STABLE_UPTIME: Duration = Duration::from_secs(30);
/// Quedas seguidas antes de desistir, como no logout, quando o compositor some.
const MAX_ATTEMPTS: u32 = 5;

fn plan(
    previous_attempt: u32,
    window_uptime: Duration,
    screen_saver: bool,
    session: bool,
) -> Option<Plan> {
    let attempt = if window_uptime >= STABLE_UPTIME {
        1
    } else {
        previous_attempt + 1
    };
    if attempt > MAX_ATTEMPTS {
        return None;
    }
    let mut args = Vec::new();
    if screen_saver {
        args.push("--screen-saver");
    }
    if session {
        args.push("--session");
    }
    Some(Plan {
        attempt,
        delay: Duration::from_secs(u64::from(attempt - 1)),
        args,
    })
}

/// Re-executa o programa com os bloqueios em vigor. Só retorna ao desistir ou
/// se o `exec` falhar; os inibidores do chamador seguem ativos durante a espera.
pub fn after_window_failure(window_uptime: Duration, screen_saver: bool, session: bool) {
    let previous_attempt = std::env::var(ATTEMPT_VAR)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let Some(plan) = plan(previous_attempt, window_uptime, screen_saver, session) else {
        eprintln!("A janela caiu {MAX_ATTEMPTS} vezes seguidas; encerrando.");
        return;
    };
    eprintln!(
        "A janela caiu; reabrindo (tentativa {} de {MAX_ATTEMPTS}).",
        plan.attempt
    );
    thread::sleep(plan.delay);
    // O caminho real mantém o nome do processo; `/proc/self/exe` cobre o caso de
    // o binário ter sido substituído por uma atualização com o programa aberto.
    let executable = std::env::current_exe()
        .ok()
        .filter(|path| path.exists())
        .unwrap_or_else(|| "/proc/self/exe".into());
    let mut command = Command::new(executable);
    if let Some(arg0) = std::env::args_os().next() {
        command.arg0(arg0);
    }
    keep_inhibitors_during_handover();
    let error = command
        .args(&plan.args)
        .env(ATTEMPT_VAR, plan.attempt.to_string())
        .exec();
    eprintln!("Não foi possível reabrir o programa: {error}");
}

/// O `exec` fecha as conexões D-Bus e, com elas, os inibidores atuais, enquanto o
/// PowerDevil só ativa uma inibição nova alguns segundos após o registro. Um neto
/// herda os descritores e os mantém abertos até o novo processo assumir.
fn keep_inhibitors_during_handover() {
    const HANDOVER_SECONDS: u32 = 15;
    // SAFETY: após o `fork` os filhos chamam apenas `fork`, `sleep` e `_exit`,
    // todas seguras em processo com várias threads. O fork duplo entrega o neto
    // ao reaper da sessão, sem deixar zumbi neste processo.
    unsafe {
        let child = libc::fork();
        if child == 0 {
            if libc::fork() == 0 {
                libc::sleep(HANDOVER_SECONDS);
            }
            libc::_exit(0);
        }
        if child > 0 {
            libc::waitpid(child, std::ptr::null_mut(), 0);
        } else {
            eprintln!(
                "Não foi possível manter os bloqueios durante a reabertura: {}",
                std::io::Error::last_os_error()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_window_relaunches_immediately_keeping_active_blocks() {
        assert_eq!(
            plan(0, Duration::from_secs(14 * 60 * 60), true, false),
            Some(Plan {
                attempt: 1,
                delay: Duration::ZERO,
                args: vec!["--screen-saver"],
            })
        );
    }

    #[test]
    fn window_failing_right_after_relaunch_retries_with_growing_delay() {
        assert_eq!(
            plan(2, Duration::from_secs(1), true, true),
            Some(Plan {
                attempt: 3,
                delay: Duration::from_secs(2),
                args: vec!["--screen-saver", "--session"],
            })
        );
    }

    #[test]
    fn gives_up_after_five_quick_failures_in_a_row() {
        assert!(plan(4, Duration::from_secs(1), true, false).is_some());
        assert_eq!(plan(5, Duration::from_secs(1), true, false), None);
    }
}
