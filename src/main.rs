#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod platform;

use clap::Parser;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use platform::PlatformInhibitor;

const APP_NAME: &str = "Block Screen Saver";
const APP_ID: &str = "io.github.ChicoFigueiredo.BlockScreenSaver";
const APP_ICON: &[u8] = include_bytes!("../assets/preferences-desktop-screensaver.ico");

/// Bloqueia recursos de inatividade e de encerramento enquanto o programa estiver aberto.
#[derive(Debug, Parser)]
#[command(name = "block-screen-saver", version, about)]
struct Args {
    /// Inicia bloqueando o protetor e o desligamento automático da tela
    #[arg(long)]
    screen_saver: bool,

    /// Inicia bloqueando logoff e encerramento de sessão
    #[arg(long)]
    session: bool,
}

struct BlockApp {
    inhibitor: PlatformInhibitor,
    screen_saver: bool,
    session: bool,
    pending_initial_state: bool,
    status: String,
}

impl BlockApp {
    fn new(args: Args) -> Self {
        Self {
            inhibitor: PlatformInhibitor::new(),
            screen_saver: args.screen_saver,
            session: args.session,
            pending_initial_state: true,
            status: "Pronto.".to_owned(),
        }
    }

    fn apply_screen_saver(&mut self, enabled: bool) {
        match self.inhibitor.set_screen_saver(enabled) {
            Ok(()) => {
                self.screen_saver = enabled;
                self.status = if enabled {
                    "Protetor/desligamento de tela bloqueado.".to_owned()
                } else {
                    "Protetor/desligamento de tela liberado.".to_owned()
                };
            }
            Err(error) => {
                // `toggle_value` já alterou o valor visual; restaura o estado
                // efetivamente aplicado quando a API do sistema recusa a ação.
                self.screen_saver = !enabled;
                self.status = format!("Não foi possível alterar o bloqueio de tela: {error}")
            }
        }
    }

    fn apply_session(&mut self, enabled: bool) {
        match self.inhibitor.set_session(enabled) {
            Ok(()) => {
                self.session = enabled;
                self.status = if enabled {
                    "Logoff e encerramento de sessão bloqueados.".to_owned()
                } else {
                    "Logoff e encerramento de sessão liberados.".to_owned()
                };
            }
            Err(error) => {
                self.session = !enabled;
                self.status = format!("Não foi possível alterar o bloqueio de sessão: {error}")
            }
        }
    }
}

impl eframe::App for BlockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // No Windows a janela já precisa existir para registrar o bloqueio de sessão.
        if self.pending_initial_state {
            self.pending_initial_state = false;
            let screen_saver = self.screen_saver;
            let session = self.session;
            if screen_saver {
                self.apply_screen_saver(true);
            }
            if session {
                self.apply_session(true);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(22)))
            .show(ctx, |ui| {
                let active_count = self.screen_saver as u8 + self.session as u8;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Proteção do sistema");
                        ui.label("Controle os bloqueios enquanto este aplicativo estiver aberto.");
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (label, color) = match active_count {
                            0 => ("INATIVO", Color32::from_rgb(100, 116, 139)),
                            1 => ("1 ATIVO", Color32::from_rgb(16, 185, 129)),
                            _ => ("2 ATIVOS", Color32::from_rgb(16, 185, 129)),
                        };
                        status_badge(ui, label, color);
                    });
                });

                ui.add_space(22.0);
                let previous_screen_saver = self.screen_saver;
                protection_switch(
                    ui,
                    &mut self.screen_saver,
                    "Tela sempre ativa",
                    "Evita o protetor e o desligamento automático da tela.",
                    "☼",
                );
                if self.screen_saver != previous_screen_saver {
                    self.apply_screen_saver(self.screen_saver);
                }

                ui.add_space(12.0);
                let previous_session = self.session;
                protection_switch(
                    ui,
                    &mut self.session,
                    "Sessão protegida",
                    "Solicita ao sistema que bloqueie encerramento e logoff.",
                    "⌁",
                );
                if self.session != previous_session {
                    self.apply_session(self.session);
                }

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(16, 185, 129), "●");
                    ui.label(&self.status);
                });
                ui.add_space(8.0);
                ui.small("Início via terminal: --screen-saver --session");
            });
    }
}

fn status_badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.7)))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.colored_label(color, egui::RichText::new(label).strong().size(11.0));
        });
}

/// Card clicável com switch próprio, para que o estado ligado/desligado seja inequívoco.
fn protection_switch(ui: &mut egui::Ui, value: &mut bool, title: &str, subtitle: &str, icon: &str) {
    let desired_size = Vec2::new(ui.available_width(), 82.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *value = !*value;
    }

    let active = *value;
    let background = if active {
        Color32::from_rgb(20, 61, 56)
    } else {
        Color32::from_rgb(31, 41, 55)
    };
    let border = if active {
        Color32::from_rgb(16, 185, 129)
    } else {
        Color32::from_rgb(71, 85, 105)
    };
    let painter = ui.painter_at(rect);
    painter.rect(
        rect,
        12.0,
        background,
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );

    let icon_rect = Rect::from_min_size(rect.min + Vec2::new(14.0, 21.0), Vec2::splat(40.0));
    painter.circle_filled(
        icon_rect.center(),
        20.0,
        if active {
            Color32::from_rgb(16, 185, 129)
        } else {
            Color32::from_rgb(71, 85, 105)
        },
    );
    painter.text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(23.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(rect.min.x + 68.0, rect.min.y + 25.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(16.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(rect.min.x + 68.0, rect.min.y + 49.0),
        Align2::LEFT_CENTER,
        subtitle,
        FontId::proportional(12.5),
        Color32::from_rgb(203, 213, 225),
    );

    let switch_size = Vec2::new(58.0, 32.0);
    let switch_rect =
        Rect::from_center_size(Pos2::new(rect.max.x - 31.0, rect.center().y), switch_size);
    let track = if active {
        Color32::from_rgb(16, 185, 129)
    } else {
        Color32::from_rgb(100, 116, 139)
    };
    painter.rect(
        switch_rect,
        16.0,
        track,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let knob_x = if active {
        switch_rect.right() - 16.0
    } else {
        switch_rect.left() + 16.0
    };
    painter.circle_filled(
        Pos2::new(knob_x, switch_rect.center().y),
        12.0,
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(switch_rect.center().x, switch_rect.max.y + 13.0),
        Align2::CENTER_CENTER,
        if active { "LIGADO" } else { "DESLIGADO" },
        FontId::proportional(10.0),
        if active {
            Color32::from_rgb(110, 231, 183)
        } else {
            Color32::from_rgb(148, 163, 184)
        },
    );
}

fn app_icon() -> egui::IconData {
    let image = image::load_from_memory_with_format(APP_ICON, image::ImageFormat::Ico)
        .expect("o ícone incorporado deve ser um arquivo ICO válido")
        .into_rgba8();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

fn main() -> eframe::Result {
    let args = Args::parse();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(APP_ID)
            .with_inner_size([560.0, 420.0])
            .with_min_inner_size([500.0, 390.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|creation_context| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(15, 23, 42);
            visuals.window_fill = Color32::from_rgb(15, 23, 42);
            visuals.override_text_color = Some(Color32::from_rgb(226, 232, 240));
            creation_context.egui_ctx.set_visuals(visuals);
            Ok(Box::new(BlockApp::new(args)))
        }),
    )
}
