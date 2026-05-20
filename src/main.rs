#[cfg(windows)]
mod app {
    use std::iter;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
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
        selected: usize, // 0 = monitor, 1 = no_kill
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
                selected: 0,
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
        crossterm::execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let result = ui_loop(&mut terminal, state);

        disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn ui_loop<B: Backend>(
        terminal: &mut Terminal<B>,
        state: &mut UiState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Bloqueia sleep imediatamente ao abrir a TUI
        prevent_sleep(state.no_monitor);

        let mut no_kill_hwnd: HWND = HWND(std::ptr::null_mut());

        loop {
            terminal.draw(|f| draw_ui(f, state))?;

            // Processa mensagens do Windows quando no_kill está ativo
            if state.no_kill {
                unsafe {
                    let mut msg = MSG::default();
                    while PeekMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
                        if msg.message == WM_QUIT {
                            state.running = false;
                            break;
                        }
                        let _ = TranslateMessage(&msg);
                        let _ = DispatchMessageW(&msg);
                    }
                }
            }

            if !state.running {
                break;
            }

            if crossterm::event::poll(Duration::from_millis(50))? {
                let prev_no_monitor = state.no_monitor;
                let prev_no_kill = state.no_kill;

                match event::read()? {
                    // Só processa Press; Release/Repeat causariam double-toggle
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
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

                // Aplica mudança de monitor imediatamente
                if state.no_monitor != prev_no_monitor {
                    prevent_sleep(state.no_monitor);
                }

                // Liga/desliga no_kill em tempo real
                if state.no_kill != prev_no_kill {
                    if state.no_kill {
                        NO_KILL_ACTIVE.store(true, Ordering::Relaxed);
                        unsafe {
                            let _ = SetConsoleCtrlHandler(Some(console_ctrl_handler), true);
                            let _ = SetProcessShutdownParameters(0x4FF, SHUTDOWN_NORETRY);
                        }
                        let class_name = wide("ScreenSaverBlockerHiddenWnd");
                        no_kill_hwnd = unsafe { create_hidden_window(&class_name) };
                        if no_kill_hwnd != HWND(std::ptr::null_mut()) {
                            let reason = wide("ScreenSaverBlocker: desligamento bloqueado!");
                            unsafe {
                                let _ = ShutdownBlockReasonCreate(no_kill_hwnd, PCWSTR(reason.as_ptr()));
                            }
                        }
                    } else {
                        NO_KILL_ACTIVE.store(false, Ordering::Relaxed);
                        unsafe {
                            let _ = SetConsoleCtrlHandler(Some(console_ctrl_handler), false);
                        }
                        if no_kill_hwnd != HWND(std::ptr::null_mut()) {
                            unsafe {
                                let _ = ShutdownBlockReasonDestroy(no_kill_hwnd);
                                let _ = DestroyWindow(no_kill_hwnd);
                            }
                            no_kill_hwnd = HWND(std::ptr::null_mut());
                        }
                    }
                }
            }
        }

        // Cleanup ao sair (ESC)
        if no_kill_hwnd != HWND(std::ptr::null_mut()) {
            unsafe {
                let _ = ShutdownBlockReasonDestroy(no_kill_hwnd);
                let _ = DestroyWindow(no_kill_hwnd);
            }
        }
        unsafe {
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }

        Ok(())
    }

    fn draw_ui(f: &mut Frame, state: &UiState) {
        let size = f.area();

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // ASCII art
                Constraint::Length(1),  // Status bar
                Constraint::Min(10),    // Options
                Constraint::Length(2),  // Help
            ])
            .split(size);

        // ── ASCII Art Title ──────────────────────────────────────────
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
        let title = Paragraph::new(title_art).alignment(Alignment::Center);
        f.render_widget(title, main_chunks[0]);

        // ── Status Bar ───────────────────────────────────────────────
        let mut active_parts: Vec<&str> = vec!["sleep"];
        if state.no_monitor { active_parts.push("monitor"); }
        if state.no_kill    { active_parts.push("anti-shutdown"); }
        let status_text = format!(
            "  [ATIVO] Bloqueando: {}  ",
            active_parts.join(" + ")
        );
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(status, main_chunks[1]);

        // ── Options ──────────────────────────────────────────────────
        let option_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(main_chunks[2]);

        // Opção 1: Monitor
        let b1_sel = state.selected == 0;
        let b1_on  = state.no_monitor;
        let b1_text = format!(
            "{}  {}  Keep Monitor Awake  {}",
            if b1_sel { "▶" } else { " " },
            if b1_on  { "●" } else { "○" },
            if b1_on  { "[ATIVO]  " } else { "[inativo]" },
        );
        let b1_style = match (b1_sel, b1_on) {
            (true,  true)  => Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD),
            (true,  false) => Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
            (false, true)  => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            (false, false) => Style::default().fg(Color::Gray),
        };
        let b1_border = if b1_on { Style::default().fg(Color::Green) }
                        else if b1_sel { Style::default().fg(Color::Blue) }
                        else { Style::default().fg(Color::DarkGray) };
        let button1 = Paragraph::new(b1_text)
            .style(b1_style)
            .block(Block::default().borders(Borders::ALL).border_style(b1_border))
            .alignment(Alignment::Center);
        f.render_widget(button1, option_chunks[0]);

        // Opção 2: No-Kill
        let b2_sel = state.selected == 1;
        let b2_on  = state.no_kill;
        let b2_text = format!(
            "{}  {}  Block Shutdown/Logoff  {}",
            if b2_sel { "▶" } else { " " },
            if b2_on  { "●" } else { "○" },
            if b2_on  { "[ATIVO]  " } else { "[inativo]" },
        );
        let b2_style = match (b2_sel, b2_on) {
            (true,  true)  => Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD),
            (true,  false) => Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
            (false, true)  => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            (false, false) => Style::default().fg(Color::Gray),
        };
        let b2_border = if b2_on { Style::default().fg(Color::Green) }
                        else if b2_sel { Style::default().fg(Color::Blue) }
                        else { Style::default().fg(Color::DarkGray) };
        let button2 = Paragraph::new(b2_text)
            .style(b2_style)
            .block(Block::default().borders(Borders::ALL).border_style(b2_border))
            .alignment(Alignment::Center);
        f.render_widget(button2, option_chunks[1]);

        // ── Help ─────────────────────────────────────────────────────
        let help = Paragraph::new(Line::from(vec![
            Span::styled("TAB", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" / "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  Navegar  •  "),
            Span::styled("SPACE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" / "),
            Span::styled("ENTER", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  Alternar  •  "),
            Span::styled("ESC", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  Sair"),
        ]))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
        f.render_widget(help, main_chunks[3]);
    }

    fn handle_key_event(key: KeyEvent, state: &mut UiState) {
        match key.code {
            KeyCode::Esc => {
                state.running = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                state.selected = (state.selected + 1) % 2;
            }
            KeyCode::BackTab | KeyCode::Up => {
                state.selected = (state.selected + 1) % 2;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match state.selected {
                    0 => state.no_monitor = !state.no_monitor,
                    1 => state.no_kill = !state.no_kill,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_event(mouse: MouseEvent, state: &mut UiState) {
        if let MouseEventKind::Down(_) = mouse.kind {
            let y = mouse.row;
            // title(8) + status(1) + margin(2) = opções começam na linha 11
            if y >= 11 && y < 14 {
                state.selected = 0;
                state.no_monitor = !state.no_monitor;
            } else if y >= 14 && y < 17 {
                state.selected = 1;
                state.no_kill = !state.no_kill;
            }
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
        let hinstance = unsafe { GetModuleHandleW(PCWSTR::null()).unwrap_or_default() };
        let mut wc = WNDCLASSEXW::default();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(hidden_wnd_proc);
        wc.hInstance = hinstance.into();
        wc.lpszClassName = PCWSTR(class_name.as_ptr());

        unsafe { let _ = RegisterClassExW(&wc); }

        unsafe {
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
                    let reason = wide("ScreenSaverBlocker: desligamento bloqueado!");
                    unsafe { let _ = ShutdownBlockReasonCreate(hwnd, PCWSTR(reason.as_ptr())); }
                    return LRESULT(0);
                }
                return LRESULT(1);
            }
            WM_ENDSESSION => {
                if NO_KILL_ACTIVE.load(Ordering::Relaxed) && wparam.0 != 0 {
                    unsafe { let _ = AbortSystemShutdownW(PCWSTR::null()); }
                    return LRESULT(0);
                }
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0); }
                return LRESULT(0);
            }
            _ => {}
        }

        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
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
