#[cfg(windows)]
mod app {
    use std::iter;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use crossterm::event::{self, Event, KeyCode};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    use windows::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, LRESULT, TRUE, WPARAM};
    use windows::Win32::System::Console::{CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Power::{
        ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
        EXECUTION_STATE, SetThreadExecutionState,
    };
    use windows::Win32::System::Shutdown::{
        AbortSystemShutdownW, ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy,
    };
    use windows::Win32::System::Threading::SetProcessShutdownParameters;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE,
        PeekMessageW, PostQuitMessage, RegisterClassExW, TranslateMessage, WM_DESTROY,
        WM_ENDSESSION, WM_QUERYENDSESSION, WM_QUIT, WNDCLASSEXW,
        HWND_MESSAGE, WINDOW_EX_STYLE,
    };
    use windows::core::PCWSTR;

    static NO_KILL_ACTIVE: AtomicBool = AtomicBool::new(false);
    const SHUTDOWN_NORETRY: u32 = 0x00000001;

    pub fn run() {
        let mut no_monitor = false;
        let mut no_kill = false;

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--no-monitor" | "-m" => no_monitor = true,
                "--no-kill" | "-k" => no_kill = true,
                "--help" | "-h" => {
                    print_usage();
                    return;
                }
                _ => {}
            }
        }

        NO_KILL_ACTIVE.store(no_kill, Ordering::Relaxed);

        prevent_sleep(no_monitor);

        if no_monitor {
            print!("ScreenSaver e Computador Travado!");
        } else {
            print!("Computador Travado!");
        }

        if no_kill {
            println!(" [--no-kill ATIVO: desligamento bloqueado]");
            run_no_kill_loop();
        } else {
            println!(" Pressione qualquer tecla para desligar o travamento...");
            let _ = enable_raw_mode();
            let _ = event::read();
            let _ = disable_raw_mode();
        }

        unsafe {
            // Retorna para o estado padrão de energia da thread.
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }
    }

    fn print_usage() {
        println!("Uso: screen-saver-blocker-rust [opcoes]");
        println!("  --no-monitor, -m  Impede o monitor de desligar");
        println!("  --no-kill, -k     Impede o computador de ser desligado");
        println!("  --help, -h        Mostra esta ajuda");
    }

    fn prevent_sleep(no_monitor: bool) {
        let base = ES_CONTINUOUS | ES_SYSTEM_REQUIRED;
        let with_away = base | ES_AWAYMODE_REQUIRED;
        let full = if no_monitor {
            with_away | ES_DISPLAY_REQUIRED
        } else {
            with_away
        };
        let fallback = if no_monitor {
            base | ES_DISPLAY_REQUIRED
        } else {
            base
        };

        unsafe {
            // Tenta away mode (Windows >= Vista), fallback se falhar.
            if SetThreadExecutionState(full) == EXECUTION_STATE(0) {
                let _ = SetThreadExecutionState(fallback);
            }
        }
    }

    fn run_no_kill_loop() {
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(console_ctrl_handler), true);
            let _ = SetProcessShutdownParameters(0x4FF, SHUTDOWN_NORETRY);
        }

        let class_name = wide("ScreenSaverBlockerHiddenWnd");
        let reason = wide("ScreenSaverBlocker: --no-kill ativo, desligamento bloqueado!");

        let hwnd = unsafe { create_hidden_window(&class_name) };

        if hwnd != HWND(std::ptr::null_mut()) {
            unsafe {
                let _ = ShutdownBlockReasonCreate(hwnd, PCWSTR(reason.as_ptr()));
            }
        }

        println!("Pressione 'q' para sair...");

        let _ = enable_raw_mode();
        let mut running = true;
        while running {
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        running = false;
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            }

            if !running {
                break;
            }

            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
                        break;
                    }
                }
            }
        }
        let _ = disable_raw_mode();

        if hwnd != HWND(std::ptr::null_mut()) {
            unsafe {
                let _ = ShutdownBlockReasonDestroy(hwnd);
                let _ = DestroyWindow(hwnd);
            }
        }
    }

    unsafe fn create_hidden_window(class_name: &[u16]) -> HWND {
        let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
        let mut wc = WNDCLASSEXW::default();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(hidden_wnd_proc);
        wc.hInstance = hinstance.into();
        wc.lpszClassName = PCWSTR(class_name.as_ptr());

        let _ = RegisterClassExW(&wc);

        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(wide("ScreenSaverBlocker").as_ptr()),
            Default::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )
        .unwrap_or(HWND(std::ptr::null_mut()))
    }

    unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> BOOL {
        if NO_KILL_ACTIVE.load(Ordering::Relaxed)
            && matches!(ctrl_type, CTRL_CLOSE_EVENT | CTRL_SHUTDOWN_EVENT | CTRL_LOGOFF_EVENT)
        {
            return TRUE;
        }

        FALSE
    }

    unsafe extern "system" fn hidden_wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_QUERYENDSESSION => {
                if NO_KILL_ACTIVE.load(Ordering::Relaxed) {
                    let reason = wide("ScreenSaverBlocker: --no-kill ativo, desligamento bloqueado!");
                    let _ = ShutdownBlockReasonCreate(hwnd, PCWSTR(reason.as_ptr()));
                    return LRESULT(0);
                }

                return LRESULT(1);
            }
            WM_ENDSESSION => {
                if NO_KILL_ACTIVE.load(Ordering::Relaxed) && wparam.0 != 0 {
                    let _ = AbortSystemShutdownW(PCWSTR::null());
                    return LRESULT(0);
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }

        DefWindowProcW(hwnd, message, wparam, lparam)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(iter::once(0)).collect()
    }
}

#[cfg(windows)]
fn main() {
    app::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Este programa suporta apenas Windows.");
}
