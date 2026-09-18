use windows::{
    Win32::{
        Foundation::{GetLastError, HWND},
        System::{
            Power::{ES_CONTINUOUS, ES_DISPLAY_REQUIRED, SetThreadExecutionState},
            Shutdown::{ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy},
            Threading::GetCurrentProcessId,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
    core::w,
};

#[derive(Default)]
pub struct PlatformInhibitor {
    screen_saver_active: bool,
    session_window: Option<HWND>,
}

impl PlatformInhibitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_screen_saver(&mut self, enabled: bool) -> Result<(), String> {
        unsafe {
            let state = if enabled {
                ES_CONTINUOUS | ES_DISPLAY_REQUIRED
            } else {
                ES_CONTINUOUS
            };
            if SetThreadExecutionState(state).0 == 0 {
                return Err(format!(
                    "SetThreadExecutionState falhou: {}",
                    GetLastError().0
                ));
            }
        }
        self.screen_saver_active = enabled;
        Ok(())
    }

    pub fn set_session(&mut self, enabled: bool) -> Result<(), String> {
        if enabled {
            if self.session_window.is_some() {
                return Ok(());
            }
            let window = current_app_window()?;
            unsafe {
                ShutdownBlockReasonCreate(window, w!("Bloqueio de sessão solicitado pelo usuário"))
                    .map_err(|error| error.to_string())?;
            }
            self.session_window = Some(window);
        } else if let Some(window) = self.session_window.take() {
            unsafe {
                ShutdownBlockReasonDestroy(window).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
}

fn current_app_window() -> Result<HWND, String> {
    unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return Err("não há uma janela ativa para registrar o bloqueio".to_owned());
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        if process_id != GetCurrentProcessId() {
            return Err("ative a janela do aplicativo e tente novamente".to_owned());
        }
        Ok(window)
    }
}

impl Drop for PlatformInhibitor {
    fn drop(&mut self) {
        let _ = self.set_session(false);
        if self.screen_saver_active {
            let _ = self.set_screen_saver(false);
        }
    }
}
