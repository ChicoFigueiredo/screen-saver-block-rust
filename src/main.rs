#[cfg(windows)]
mod app {
    use std::iter;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
    use ratatui::{
        backend::{Backend, CrosstermBackend},
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph},
        Frame, Terminal,
    };
    use std::io;

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

    struct UiState {
        no_monitor: bool,
        no_kill: bool,
        selected_button: usize,
        running: bool,
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            print_usage();
            return;
        }

        let mut state = UiState {
            no_monitor: args.iter().any(|arg| arg == "--no-monitor" || arg == "-m"),
            no_kill: args.iter().any(|arg| arg == "--no-kill" || arg == "-k"),
            selected_button: 0,
            running: true,
        };

        if args.len() > 1 && !args.iter().any(|arg| arg == "--ui") {
            run_blocking_mode(state.no_monitor, state.no_kill);
        } else {
            if let Err(e) = run_tui(&mut state) {
                eprintln!("TUI Error: {}", e);
            }
        }
    }

    fn print_usage() {
        println!("Uso: screen-saver-blocker-rust [opcoes]");
        println!("  --no-monitor, -m  Impede o monitor de desligar");
        println!("  --no-kill, -k     Impede o computador de ser desligado");
        println!("  --ui               Força modo TUI interativo");
        println!("  --help, -h        Mostra esta ajuda");
    }

    fn run_tui(state: &mut UiState) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let result = ui_loop(&mut terminal, state);

        disable_raw_mode()?;
        crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn ui_loop<B: Backend>(terminal: &mut Terminal<B>, state: &mut UiState) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| {
                draw_ui(f, state);
            })?;

            if crossterm::event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        handle_key_event(key, state);
                        if !state.running {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => {
                        handle_mouse_event(mouse, state);
                        if !state.running {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        if state.running {
            state.running = false;
            NO_KILL_ACTIVE.store(state.no_kill, Ordering::Relaxed);
            run_blocking_mode(state.no_monitor, state.no_kill);
        }

        Ok(())
    }

    fn draw_ui(f: &mut Frame, state: &UiState) {
        let size = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                ]
                .as_ref(),
            )
            .split(size);

        let title = Paragraph::new("Screen Saver Blocker")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        let subtitle = Paragraph::new("Keep your computer awake")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(subtitle, chunks[1]);

        let control_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(chunks[2]);

        let button1_style = if state.selected_button == 0 {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let button1_text = if state.no_monitor {
            "[✓] Keep Monitor Awake (ON)"
        } else {
            "[ ] Keep Monitor Awake (OFF)"
        };
        let button1 = Paragraph::new(button1_text)
            .style(button1_style)
            .block(Block::default().borders(Borders::ALL).title("Option 1"))
            .alignment(Alignment::Center);
        f.render_widget(button1, control_chunks[0]);

        let button2_style = if state.selected_button == 1 {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let button2_text = if state.no_kill {
            "[✓] Block Shutdown/Logoff (ON)"
        } else {
            "[ ] Block Shutdown/Logoff (OFF)"
        };
        let button2 = Paragraph::new(button2_text)
            .style(button2_style)
            .block(Block::default().borders(Borders::ALL).title("Option 2"))
            .alignment(Alignment::Center);
        f.render_widget(button2, control_chunks[1]);

        let button3_style = if state.selected_button == 2 {
            Style::default().bg(Color::Green).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let button3 = Paragraph::new("[ START ]")
            .style(button3_style)
            .block(Block::default().borders(Borders::ALL).title("Action"))
            .alignment(Alignment::Center);
        f.render_widget(button3, control_chunks[2]);

        let instructions = vec![
            Line::from(vec![
                Span::styled("TAB", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" or "),
                Span::styled("Arrows", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" • "),
                Span::styled("Space/Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" • "),
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to quit"),
            ]),
        ];
        let help = Paragraph::new(instructions)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help, control_chunks[3]);
    }

    fn handle_key_event(key: KeyEvent, state: &mut UiState) {
        match key.code {
            KeyCode::Esc => {
                state.running = false;
            }
            KeyCode::Tab => {
                state.selected_button = (state.selected_button + 1) % 3;
            }
            KeyCode::BackTab => {
                state.selected_button = if state.selected_button == 0 {
                    2
                } else {
                    state.selected_button - 1
                };
            }
            KeyCode::Up => {
                state.selected_button = if state.selected_button == 0 {
                    2
                } else {
                    state.selected_button - 1
                };
            }
            KeyCode::Down => {
                state.selected_button = (state.selected_button + 1) % 3;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match state.selected_button {
                    0 => state.no_monitor = !state.no_monitor,
                    1 => state.no_kill = !state.no_kill,
                    2 => state.running = false,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_event(mouse: MouseEvent, state: &mut UiState) {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                let y = mouse.row;
                if y >= 7 && y < 10 {
                    state.selected_button = 0;
                    state.no_monitor = !state.no_monitor;
                } else if y >= 10 && y < 13 {
                    state.selected_button = 1;
                    state.no_kill = !state.no_kill;
                } else if y >= 13 && y < 16 {
                    state.selected_button = 2;
                    state.running = false;
                }
            }
            _ => {}
        }
    }

    fn run_blocking_mode(no_monitor: bool, no_kill: bool) {
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
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }
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
