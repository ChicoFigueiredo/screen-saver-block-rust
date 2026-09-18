use zbus::{blocking::Connection, zvariant::OwnedFd};

const APP_ID: &str = "block-screen-saver";
const REASON: &str = "Bloqueio solicitado pelo usuário";

struct ScreenSaverLease {
    connection: Connection,
    cookie: u32,
}

/// Mantém os leases D-Bus vivos. O `OwnedFd` do logind é o próprio bloqueio:
/// fechá-lo libera a operação novamente.
#[derive(Default)]
pub struct PlatformInhibitor {
    screen_saver: Option<ScreenSaverLease>,
    session_lock: Option<OwnedFd>,
}

impl PlatformInhibitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_screen_saver(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            if self.screen_saver.is_some() {
                return Ok(());
            }
            let connection = Connection::session().map_err(|error| error.to_string())?;
            let proxy = zbus::blocking::Proxy::new(
                &connection,
                "org.freedesktop.ScreenSaver",
                "/ScreenSaver",
                "org.freedesktop.ScreenSaver",
            )
            .map_err(|error| error.to_string())?;
            let cookie: u32 = proxy
                .call("Inhibit", &(APP_ID, REASON))
                .map_err(|error| error.to_string())?;
            self.screen_saver = Some(ScreenSaverLease { connection, cookie });
        } else if let Some(lease) = self.screen_saver.take() {
            let proxy = zbus::blocking::Proxy::new(
                &lease.connection,
                "org.freedesktop.ScreenSaver",
                "/ScreenSaver",
                "org.freedesktop.ScreenSaver",
            )
            .map_err(|error| error.to_string())?;
            proxy
                .call::<_, _, ()>("UnInhibit", &(lease.cookie))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn set_session(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            if self.session_lock.is_some() {
                return Ok(());
            }
            let connection = Connection::system().map_err(|error| error.to_string())?;
            let proxy = zbus::blocking::Proxy::new(
                &connection,
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
            )
            .map_err(|error| error.to_string())?;
            let lock: OwnedFd = proxy
                .call("Inhibit", &("shutdown", APP_ID, REASON, "block"))
                .map_err(|error| error.to_string())?;
            self.session_lock = Some(lock);
        } else {
            self.session_lock = None;
        }
        Ok(())
    }
}

impl Drop for PlatformInhibitor {
    fn drop(&mut self) {
        // A tentativa de UnInhibit é útil para o serviço; caso o processo esteja
        // encerrando, o D-Bus também removerá o inibidor pela desconexão.
        let _ = self.set_screen_saver(false);
        self.session_lock = None;
    }
}
