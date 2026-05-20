#[cfg(windows)]
mod app {
    use std::iter;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
    use ratatui::{
        backend::{Backend, CrosstermBackend},
        layout::{Alignment, Constraint, Direction, Layout},
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

        // Se há argumentos (que não sejam --help), executar modo CLI direto
        // Senão, abrir TUI interativa
        if args.len() > 1 {
            let no_monitor = args.iter().any(|arg| arg == "--no-monitor" || arg == "-m");
            let no_kill = args.iter().any(|arg| arg == "--no-kill" || arg == "-k");
            NO_KILL_ACTIVE.store(no_kill, Ordering::Relaxed);
            run_blocking_mode(no_monitor, no_kill);
        } else {
            let mut state = UiState {
                no_monitor: false,
                no_kill: false,
                selected_button: 0,
                running: true,
            };
            if let Err(e) = run_tui(&mut state) {
                eprintln!("TUI Error: {}", e);
            }
        }
    }

    fn print_usage() {
        println!("Screen Saver Blocker v0.8.1");
        println!();
        println!("Uso: screen-saver-blocker-rust [opcoes]");
        println!();
        println!("Opcoes:");
        println!("  --no-monitor, -m   Impede o monitor de desligar");
        println!("  --no-kill, -k      Impede o computador de ser desligado");
        println!("  --help, -h         Mostra esta ajuda");
        println!();
        println!("Exemplos:");
        println!("  screen-saver-blocker-rust              # Abre interface TUI");
        println!("  screen-saver-blocker-rust --no-monitor # Mantém monitor ligado");
        println!("  screen-saver-blocker-rust --no-kill    # Bloqueia desligamento");
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

            if crossterm::event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) => {
                        handle_key_event(key, state);
                    }
                    Event::Mouse(mouse) => {
                        handle_mouse_event(mouse, state);
                    }
                    _ => {}
                }
                
                if !state.running {
                    break;
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

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(12),
                Constraint::Length(2),
            ])
            .split(size);

        // ASCII Art Title
        let title_art = vec![
            Line::from(vec![
                Span::styled("╔═══════════════════════════════════════════╗", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║     ", Style::default().fg(Color::Cyan)),
                Span::styled("███████╗ ██████╗ ███████╗███████╗██╗", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║     ", Style::default().fg(Color::Cyan)),
                Span::styled("██╔════╝██╔════╝ ██╔════╝██╔════╝██║", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║     ", Style::default().fg(Color::Cyan)),
                Span::styled("███████╗██║  ███╗███████╗█████╗  ██║", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║     ", Style::default().fg(Color::Cyan)),
                Span::styled("╚════██║██║   ██║╚════██║██╔══╝  ██║", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║     ", Style::default().fg(Color::Cyan)),
                Span::styled("███████║╚██████╔╝███████║███████╗██║", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("║", Style::default().fg(Color::Cyan)),
                Span::raw("  Keep your computer awake "),
                Span::styled("           ║", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("╚═══════════════════════════════════════════╝", Style::default().fg(Color::Cyan)),
            ]),
        ];
        let title = Paragraph::new(title_art)
            .alignment(Alignment::Center);
        f.render_widget(title, main_chunks[0]);

        // Options section
        let option_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(3)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(2),
            ])
            .split(main_chunks[1]);

        // Button 1: Monitor
        let b1_selected = state.selected_button == 0;
        let b1_state = if state.no_monitor { "●" } else { "○" };
        let b1_label = format!("{}  Keep Monitor Awake", b1_state);
        let b1_style = if b1_selected {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let b1_text = if b1_selected {
            format!("  ▶ {}  ◀  ", b1_label)
        } else {
            format!("    {}    ", b1_label)
        };
        let button1 = Paragraph::new(b1_text)
            .style(b1_style)
            .block(Block::default().borders(Borders::ALL).border_style(
                if b1_selected { Style::default().fg(Color::Blue) } else { Style::default().fg(Color::DarkGray) }
            ))
            .alignment(Alignment::Center);
        f.render_widget(button1, option_chunks[0]);

        // Button 2: Kill
        let b2_selected = state.selected_button == 1;
        let b2_state = if state.no_kill { "●" } else { "○" };
        let b2_label = format!("{}  Block Shutdown/Logoff", b2_state);
        let b2_style = if b2_selected {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let b2_text = if b2_selected {
            format!("  ▶ {}  ◀  ", b2_label)
        } else {
            format!("    {}    ", b2_label)
        };
        let button2 = Paragraph::new(b2_text)
            .style(b2_style)
            .block(Block::default().borders(Borders::ALL).border_style(
                if b2_selected { Style::default().fg(Color::Blue) } else { Style::default().fg(Color::DarkGray) }
            ))
            .alignment(Alignment::Center);
        f.render_widget(button2, option_chunks[1]);

        // Button 3: Start
        let b3_selected = state.selected_button == 2;
        let b3_style = if b3_selected {
            Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let b3_text = if b3_selected {
            "  ▶ ★  START  ★  ◀  ".to_string()
        } else {
            "    ★  START  ★    ".to_string()
        };
        let button3 = Paragraph::new(b3_text)
            .style(b3_style)
            .block(Block::default().borders(Borders::ALL).border_style(
                if b3_selected { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) }
            ))
            .alignment(Alignment::Center);
        f.render_widget(button3, option_chunks[2]);

        // Instructions
        let instructions = vec![
            Line::from(vec![
                Span::styled("TAB", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                Span::raw(" / "),
                Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                Span::raw("  Navigate  •  "),
                Span::styled("SPACE", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                Span::raw(" / "),
                Span::styled("ENTER", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                Span::raw("  Toggle  •  "),
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                Span::raw("  Quit"),
            ]),
        ];
        let help = Paragraph::new(instructions)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help, option_chunks[3]);

        // Footer
        let footer = Paragraph::new("v0.8.1")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        f.render_widget(footer, main_chunks[2]);
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
            KeyCode::Enter => {
                match state.selected_button {
                    0 => state.no_monitor = !state.no_monitor,
                    1 => state.no_kill = !state.no_kill,
                    2 => state.running = false,
                    _ => {}
                }
            }
            KeyCode::Char(' ') => {
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
