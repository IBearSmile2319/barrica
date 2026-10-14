//! Ventana «Acerca de»: versión, motor, sistema, versiones nuevas, licencias y créditos.

use super::discover::{diagonal, gradient, shadow};
use super::*;
use eframe::egui::Id;

/// Lo que Barrica usa y a quién se lo debe (nombre, para qué, licencia, web).
const CREDITS: &[(&str, &str, &str, &str)] = &[
    ("Wine", "el motor: ejecuta programas de Windows", "LGPL-2.1+", "https://www.winehq.org"),
    ("CodeWeavers", "el código fuente libre de CrossOver, base del motor", "LGPL-2.1+", "https://www.codeweavers.com/crossover/source"),
    ("DXMT", "DirectX 10 y 11 sobre Metal", "MIT", "https://github.com/3Shain/dxmt"),
    ("MoltenVK", "Vulkan sobre Metal", "Apache-2.0", "https://github.com/KhronosGroup/MoltenVK"),
    ("Wine Mono y Gecko", ".NET y HTML dentro de los bottles", "varias", "https://gitlab.winehq.org/wine-mono/wine-mono"),
    ("winget-pkgs", "versiones de las apps del catálogo", "MIT", "https://github.com/microsoft/winget-pkgs"),
    ("Simple Icons", "logos del catálogo", "CC0-1.0", "https://simpleicons.org"),
    ("egui", "la interfaz", "MIT / Apache-2.0", "https://github.com/emilk/egui"),
];

impl App {
    pub(super) fn open_about(&mut self) {
        self.about = true;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::AboutInfo(about::gather()));
            ctx.request_repaint();
        });
    }

    pub(super) fn check_updates(&mut self) {
        self.release = None;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Release(about::latest_release()));
            ctx.request_repaint();
        });
    }

    pub(super) fn about_view(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.about {
            return;
        }
        let window = ctx.style_of(ctx.theme()).visuals.window_fill;
        let frame = egui::Frame::new().fill(window).corner_radius(20).inner_margin(Margin::ZERO);
        let modal = egui::Modal::new(Id::new("about")).frame(frame).show(ctx, |ui| {
            ui.set_width(540.0);
            ui.set_max_width(540.0);
            let p = pal(ui);

            // Cabecera: el barril sobre el degradado vino.
            let (band, _) = ui.allocate_exact_size(vec2(540.0, 150.0), Sense::hover());
            let top = Rect::from_min_max(band.min, pos2(band.right(), band.bottom() + 20.0));
            gradient(ui.painter(), top, 20.0, diagonal(top, Color32::from_rgb(0x9A, 0x1F, 0x48), Color32::from_rgb(0x24, 0x0A, 0x1E)));
            ui.painter().rect_filled(Rect::from_min_max(pos2(band.left(), band.bottom()), top.max), 0, window);
            let tile = Rect::from_min_size(band.min + vec2(28.0, 34.0), vec2(82.0, 82.0));
            shadow(ui.painter(), tile, 22, 90);
            ui.painter().rect_filled(tile, 22, Color32::from_rgb(0x2B, 0x10, 0x1C));
            barrel(ui.painter(), tile.shrink(12.0));
            let x = tile.right() + 20.0;
            ui.painter().text(pos2(x, tile.top() + 26.0), Align2::LEFT_CENTER, "Barrica", FontId::new(30.0, demi()), Color32::WHITE);
            ui.painter().text(
                pos2(x, tile.top() + 58.0),
                Align2::LEFT_CENTER,
                format!("Versión {}", about::version_line()),
                FontId::proportional(14.0),
                Color32::from_white_alpha(210),
            );
            ui.painter().text(
                pos2(x, tile.top() + 80.0),
                Align2::LEFT_CENTER,
                "Juegos y apps de Windows en tu Mac",
                FontId::proportional(13.0),
                Color32::from_white_alpha(150),
            );
            let close = Rect::from_center_size(pos2(band.right() - 24.0, band.top() + 24.0), vec2(28.0, 28.0));
            let close_resp = ui.interact(close, Id::new("about-close"), Sense::click());
            let dim = if close_resp.hovered() { 90 } else { 50 };
            ui.painter().circle_filled(close.center(), 14.0, Color32::from_black_alpha(dim));
            let (c, k) = (close.center(), 4.5);
            for d in [vec2(k, k), vec2(k, -k)] {
                ui.painter().line_segment([c - d, c + d], Stroke::new(1.8, Color32::WHITE));
            }
            if close_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.about = false;
            }

            let max = (ui.ctx().content_rect().height() - 260.0).max(160.0);
            egui::ScrollArea::vertical().max_height(max).min_scrolled_height(max).max_width(540.0).show(ui, |ui| {
            egui::Frame::new().inner_margin(Margin { left: 26, right: 26, top: 4, bottom: 22 }).show(ui, |ui| {
                self.update_row(ui, actions);
                ui.add_space(12.0);

                // Motor y sistema
                egui::Frame::new().fill(p.well).corner_radius(14).inner_margin(Margin::symmetric(16, 12)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match &self.about_info {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(RichText::new("Leyendo el motor…").color(p.muted));
                            });
                        }
                        Some(info) => {
                            egui::Grid::new("about-grid").num_columns(2).spacing([22.0, 8.0]).show(ui, |ui| {
                                let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                                    ui.label(RichText::new(k).color(p.muted));
                                    ui.label(RichText::new(v).color(p.text));
                                    ui.end_row();
                                };
                                row(ui, "Motor", info.engine.as_deref().unwrap_or("Ninguno: elígelo en «Motor»"));
                                if let Some(w) = &info.wine {
                                    row(ui, "Wine", w);
                                }
                                if !info.parts.is_empty() {
                                    let parts: Vec<String> = info.parts.iter().map(|(k, v)| format!("{k} {v}")).collect();
                                    ui.label(RichText::new("Componentes").color(p.muted));
                                    ui.add(egui::Label::new(RichText::new(parts.join(" · ")).color(p.text)).wrap());
                                    ui.end_row();
                                }
                                row(ui, "D3DMetal", info.d3dmetal.as_deref().unwrap_or("No instalado (barrica gptk)"));
                                let rosetta = if info.rosetta { "con Rosetta" } else { "sin Rosetta" };
                                row(ui, "Sistema", &format!("{} · {} · {rosetta}", info.macos, info.chip));
                                let cat = format!("{} apps, del {}", self.catalog.apps.len(), self.catalog.updated);
                                row(ui, "Catálogo", &cat);
                            });
                        }
                    }
                });
                ui.add_space(12.0);

                ui.horizontal_wrapped(|ui| {
                    if ui.button("Sitio web").clicked() {
                        actions.push(Action::Open(about::WEB.into()));
                    }
                    if ui.button("GitHub").clicked() {
                        actions.push(Action::Open(about::repo_url().into()));
                    }
                    if ui.button("Reportar un problema").clicked() {
                        actions.push(Action::Open(format!("{}/issues/new", about::repo_url()).into()));
                    }
                    if ui.button("Copiar información").on_hover_text("Versión, motor y sistema, para un reporte").clicked() {
                        let text = self.about_info.as_ref().map(about::report).unwrap_or_else(about::version_line);
                        ui.ctx().copy_text(text);
                        self.ok("Información copiada.");
                    }
                });
                ui.add_space(14.0);

                ui.label(RichText::new("Licencia").family(demi()).color(p.text));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new("Barrica es software libre bajo la").small().color(p.muted));
                    if ui.link(RichText::new(about::LICENSE).small()).clicked() {
                        actions.push(Action::Open(format!("{}/blob/master/LICENSE", about::repo_url()).into()));
                    }
                    ui.label(RichText::new("; el motor es Wine, bajo la LGPL-2.1+. Detalle en los").small().color(p.muted));
                    if ui.link(RichText::new("avisos de terceros").small()).clicked() {
                        actions.push(Action::Open(about::notices()));
                    }
                    ui.label(RichText::new(".").small().color(p.muted));
                });
                ui.add_space(10.0);

                ui.label(RichText::new("Gracias a").family(demi()).color(p.text));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (i, (name, what, license, url)) in CREDITS.iter().enumerate() {
                        if i > 0 {
                            ui.label(RichText::new("·").small().color(p.muted));
                        }
                        let link = ui.link(RichText::new(*name).small()).on_hover_text(format!("{what} ({license})"));
                        if link.clicked() {
                            actions.push(Action::Open((*url).into()));
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new(
                        "Barrica no está afiliada a CodeWeavers, Apple ni a ninguna de las marcas del catálogo. \
                         D3DMetal es de Apple y no viene con Barrica.",
                    )
                    .small()
                    .color(p.muted),
                );
            });
            });
        });
        if modal.should_close() {
            self.about = false;
        }
    }

    fn update_row(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = pal(ui);
        ui.horizontal(|ui| {
            match &self.release {
                None => {
                    ui.spinner();
                    ui.label(RichText::new("Buscando versiones nuevas…").color(p.muted));
                }
                Some(Ok(r)) if r.newer => {
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, WINE);
                    ui.label(RichText::new(format!("Barrica {} está disponible", r.version)).family(demi()).color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if primary(ui, "Descargar", true).clicked() {
                            actions.push(Action::Open(r.url.clone().into()));
                        }
                    });
                }
                Some(Ok(_)) => {
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, RUNNING);
                    ui.label(RichText::new("Tienes la última versión").color(p.text));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new("No se pudo comprobar si hay versiones nuevas.").small().color(p.muted)).on_hover_text(e);
                    if ui.small_button("Reintentar").clicked() {
                        actions.push(Action::CheckUpdates);
                    }
                }
            }
        });
    }
}
