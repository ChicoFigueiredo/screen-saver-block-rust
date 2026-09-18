//! Pontes específicas de plataforma. A interface gráfica permanece agnóstica
//! ao gerenciador de janelas; somente esta camada conhece cada sistema operacional.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::PlatformInhibitor;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::PlatformInhibitor;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod unsupported {
    #[derive(Default)]
    pub struct PlatformInhibitor;

    impl PlatformInhibitor {
        pub fn new() -> Self {
            Self
        }

        pub fn set_screen_saver(&mut self, _enabled: bool) -> Result<(), String> {
            Err("sistema operacional não suportado".to_owned())
        }

        pub fn set_session(&mut self, _enabled: bool) -> Result<(), String> {
            Err("sistema operacional não suportado".to_owned())
        }
    }
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use unsupported::PlatformInhibitor;
