//! «Descubrir»: el catálogo de apps para instalar, con sus logos, su ficha y el progreso de cada instalación.

use super::*;
use crate::catalog::{App as CatalogApp, Target};
use eframe::egui::{Id, Pos2, text::LayoutJob};

// ---------- Color y formas ----------

/// `#RRGGBB` → color; si no se entiende, el vino de Barrica.
pub(super) fn hex(s: &str) -> Color32 {
    let v = s.strip_prefix('#').and_then(|h| u32::from_str_radix(h, 16).ok()).filter(|_| s.len() == 7);
    v.map_or(WINE, |v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn luminance(c: Color32) -> f32 {
    (0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) / 255.0
}

/// Blanco sobre colores oscuros, casi negro sobre los claros (Rockstar, Notepad++…).
fn on(c: Color32) -> Color32 {
    if luminance(c) > 0.62 { Color32::from_rgb(0x1A, 0x17, 0x1C) } else { Color32::WHITE }
}

/// Rectángulo redondeado con degradado: un abanico desde el centro reproduce exacto cualquier degradado lineal.
fn gradient(painter: &egui::Painter, rect: Rect, radius: f32, color_at: impl Fn(Pos2) -> Color32) {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut ring = Vec::new();
    for (corner, start) in [
        (pos2(rect.right() - r, rect.top() + r), -90.0f32),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), 90.0),
        (pos2(rect.left() + r, rect.top() + r), 180.0),
    ] {
        for i in 0..=8 {
            let a = (start + 90.0 * i as f32 / 8.0).to_radians();
            ring.push(corner + vec2(a.cos(), a.sin()) * r);
        }
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), color_at(rect.center()));
    for p in &ring {
        mesh.colored_vertex(*p, color_at(*p));
    }
    let n = ring.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(mesh);
}

/// Degradado de arriba-izquierda (`a`) a abajo-derecha (`b`).
fn diagonal(rect: Rect, a: Color32, b: Color32) -> impl Fn(Pos2) -> Color32 {
    move |p: Pos2| {
        let t = ((p.x - rect.left()) / rect.width() * 0.65 + (p.y - rect.top()) / rect.height() * 0.35).clamp(0.0, 1.0);
        mix(a, b, t)
    }
}

fn shadow(painter: &egui::Painter, rect: Rect, radius: u8, strength: u8) {
    let s = egui::epaint::Shadow { offset: [0, 6], blur: 22, spread: 0, color: Color32::from_black_alpha(strength) };
    painter.add(s.as_shape(rect, radius));
}

/// Iniciales para las apps sin logo: «Amazon Games» → «AG».
fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = |w: &str| w.chars().next().unwrap_or('?').to_uppercase().to_string();
    match words.as_slice() {
        [a, b, ..] if b.chars().next().is_some_and(char::is_alphanumeric) => first(a) + &first(b),
        [a, ..] => first(a),
        [] => "?".into(),
    }
}

/// El icono de una app: su color de marca con un brillo suave y el logo (o las iniciales) encima.
pub(super) fn brand_icon(ui: &egui::Ui, rect: Rect, app: &CatalogApp, logo: Option<&PathBuf>) {
    let c = hex(&app.color);
    let radius = rect.width() * 0.26;
    gradient(ui.painter(), rect, radius, diagonal(rect, mix(c, Color32::WHITE, 0.14), mix(c, Color32::BLACK, 0.12)));
    glyph(ui, rect.shrink(rect.width() * 0.2), app, logo, on(c));
}

/// Fondo del banner de un bottle con la marca de su app: degradado y el logo grande a la derecha.
pub(super) fn brand_banner(ui: &egui::Ui, rect: Rect, app: &CatalogApp, logo: Option<&PathBuf>) {
    let c = hex(&app.color);
    gradient(&ui.painter_at(rect), rect, 0.0, diagonal(rect, mix(c, Color32::WHITE, 0.08), mix(c, Color32::BLACK, 0.6)));
    let size = rect.height() * 0.78;
    let mark = Rect::from_center_size(pos2(rect.right() - size * 0.75, rect.center().y), vec2(size, size));
    let faded = if on(c) == Color32::WHITE { Color32::from_white_alpha(36) } else { Color32::from_black_alpha(36) };
    glyph(ui, mark, app, logo, faded);
}

fn glyph(ui: &egui::Ui, rect: Rect, app: &CatalogApp, logo: Option<&PathBuf>, color: Color32) {
    match logo {
        Some(path) => egui::Image::new(uri(path)).tint(color).paint_at(ui, rect),
        None => {
            let text = initials(&app.name);
            let size = rect.height() * if text.chars().count() > 1 { 0.62 } else { 0.8 };
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(size, demi()), color);
        }
    }
}

/// Texto en un ancho dado, con «…» si no cabe en `rows` líneas.
fn text_block(ui: &egui::Ui, text: &str, font: FontId, color: Color32, width: f32, rows: usize) -> Arc<egui::Galley> {
    let mut job = LayoutJob::simple(text.to_string(), font, color, width);
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    ui.painter().layout_job(job)
}

/// Botón con forma de píldora pintado a mano (dentro de las tarjetas).
fn pill(ui: &mut egui::Ui, rect: Rect, id: Id, text: &str, fill: Color32, color: Color32) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let fill = if resp.hovered() { mix(fill, if luminance(fill) > 0.5 { Color32::BLACK } else { Color32::WHITE }, 0.12) } else { fill };
    ui.painter().rect_filled(rect, rect.height() / 2.0, fill);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(14.0, demi()), color);
    resp
}

/// Barra de progreso en forma de píldora; sin fracción conocida, un tramo que va y viene.
fn progress_pill(ui: &egui::Ui, rect: Rect, fraction: Option<f32>, color: Color32) {
    let painter = ui.painter();
    let track = if ui.visuals().dark_mode { Color32::from_white_alpha(22) } else { Color32::from_black_alpha(18) };
    painter.rect_filled(rect, rect.height() / 2.0, track);
    let inner = rect.shrink(3.0);
    let bar = match fraction {
        Some(f) => Rect::from_min_size(inner.min, vec2((inner.width() * f).max(inner.height()), inner.height())),
        None => {
            let t = ui.input(|i| i.time) as f32;
            let w = inner.width() * 0.35;
            let x = inner.left() + (inner.width() - w) * (0.5 + 0.5 * (t * 2.2).sin());
            Rect::from_min_size(pos2(x, inner.top()), vec2(w, inner.height()))
        }
    };
    painter.rect_filled(bar, inner.height() / 2.0, color);
    ui.ctx().request_repaint();
}

/// Estrella de cuatro puntas (el icono de «Descubrir»): dos rombos cruzados.
fn sparkle(painter: &egui::Painter, c: Pos2, r: f32, color: Color32) {
    let thin = r * 0.28;
    for (a, b) in [(vec2(0.0, r), vec2(thin, 0.0)), (vec2(r, 0.0), vec2(0.0, thin))] {
        let pts = vec![c - a, c + b, c + a, c - b];
        painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    }
}

// ---------- Estado de cada app ----------

enum State<'a> {
    Idle,
    Installing(&'a str, &'a Step),
    Installed(&'a str),
}

impl App {
    fn state(&self, id: &str) -> State<'_> {
        if let Some((bottle, step)) = self.installs.get(id) {
            State::Installing(bottle, step)
        } else if let Some((bottle, _)) = self.found.get(id) {
            State::Installed(bottle)
        } else {
            State::Idle
        }
    }

    /// La app del catálogo de un bottle: la que tiene instalada o para la que se creó.
    pub(super) fn bottle_app(&self, b: &Bottle) -> Option<&CatalogApp> {
        let id = self.found.iter().find(|(_, (bottle, _))| bottle == &b.name).map(|(id, _)| id.as_str());
        id.and_then(|id| self.catalog.app(id))
            .or_else(|| self.catalog.apps.iter().find(|a| !b.config.description.is_empty() && a.name == b.config.description))
    }

    /// La app del catálogo que corresponde a un programa de la biblioteca, para usar su logo.
    pub(super) fn brand_for(&self, p: &Program) -> Option<&CatalogApp> {
        let path = p.path.to_string_lossy().to_lowercase();
        self.catalog.apps.iter().find(|a| {
            a.name.eq_ignore_ascii_case(&p.name) || a.exe.iter().any(|e| !e.contains('*') && path.ends_with(&e.to_lowercase()))
        })
    }

    // ---------- Acciones ----------

    pub(super) fn show_app(&mut self, id: String) {
        let Some(app) = self.catalog.app(&id) else { return };
        self.target_name = unique(&self.bottles, &app.name);
        // Sin motor propio solo se puede instalar en un bottle que ya existe (p. ej. uno vinculado a CrossOver).
        self.target_bottle = if self.engine.is_some() { None } else { self.bottles.first().map(|b| b.name.clone()) };
        self.detail = Some(id);
    }

    pub(super) fn install(&mut self, id: String) {
        let Some(app) = self.catalog.app(&id).cloned() else { return };
        if self.installs.contains_key(&id) {
            return;
        }
        let target = match &self.target_bottle {
            Some(b) if self.detail.as_deref() == Some(id.as_str()) => Target::Existing(b.clone()),
            _ if self.detail.as_deref() == Some(id.as_str()) => Target::New(self.target_name.trim().to_string()),
            // Desde la tarjeta: bottle nuevo con el nombre de la app (o el único posible si no hay motor).
            _ => match (&self.engine, self.bottles.first()) {
                (None, Some(b)) => Target::Existing(b.name.clone()),
                _ => Target::New(unique(&self.bottles, &app.name)),
            },
        };
        let bottle = match &target {
            Target::New(n) | Target::Existing(n) => n.clone(),
        };
        if matches!(target, Target::New(_)) && self.engine.is_none() {
            self.detail = Some(id);
            return self.fail("Para un bottle nuevo hace falta un motor Wine: elígelo en «Motor», arriba.");
        }
        self.installs.insert(id.clone(), (bottle, Step::Resolving));
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let result = catalog::install(&app, &target, &|step| {
                let _ = tx.send(Msg::Step(id.clone(), step));
                ctx.request_repaint();
            });
            let _ = tx.send(Msg::InstallDone(id, result));
            ctx.request_repaint();
        });
    }

    pub(super) fn install_done(&mut self, id: String, result: cx::Result<catalog::Outcome>) {
        self.installs.remove(&id);
        let name = self.catalog.app(&id).map_or(id.clone(), |a| a.name.clone());
        let (title, body, error) = match result {
            Ok(o) => {
                let version = o.version.map(|v| format!(" {v}")).unwrap_or_default();
                let mut body = format!("{name}{version} quedó instalada en «{}».", o.bottle);
                for w in o.warnings {
                    body += &format!("\n{w}");
                }
                self.ok(format!("{name} instalada en «{}».", o.bottle));
                (format!("{name} está lista"), body, false)
            }
            Err(e) => {
                self.fail(format!("{name}: {e}"));
                (format!("No se pudo instalar {name}"), e, true)
            }
        };
        self.toast = Some(Toast { app: id, title, body, error, at: Instant::now() });
        self.refresh();
    }

    pub(super) fn open_app(&mut self, id: &str) {
        let Some((bottle, exe)) = self.found.get(id).cloned() else { return };
        let name = self.catalog.app(id).map_or(id.to_string(), |a| a.name.clone());
        match cx::get(&bottle).and_then(|b| cx::launch(&b, &exe, &[], None)) {
            Ok((child, log)) => {
                self.ok(format!("Abriendo «{name}»…"));
                self.polled = Instant::now() - Duration::from_secs(60);
                self.watch(child, name, log);
            }
            Err(e) => self.fail(e),
        }
    }

    // ---------- Vistas ----------

    /// «Descubrir» en la barra lateral. Devuelve si se pulsó.
    pub(super) fn discover_nav(&self, ui: &mut egui::Ui) -> bool {
        let p = pal(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
        let selected = self.discover && !self.creating;
        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, 10, WINE);
        } else if resp.hovered() {
            painter.rect_filled(rect, 10, p.card_hover);
        }
        let tile = Rect::from_min_size(rect.min + vec2(9.0, 8.0), vec2(30.0, 30.0));
        let (a, b) = if selected {
            (Color32::from_white_alpha(70), Color32::from_white_alpha(30))
        } else {
            (Color32::from_rgb(0xD2, 0x3A, 0x6A), Color32::from_rgb(0x6E, 0x14, 0x33))
        };
        gradient(painter, tile, 9.0, diagonal(tile, a, b));
        sparkle(painter, tile.center(), 9.0, Color32::WHITE);
        let color = if selected { Color32::WHITE } else { p.text };
        painter.text(pos2(rect.left() + 50.0, rect.center().y), Align2::LEFT_CENTER, "Descubrir", FontId::new(15.0, demi()), color);
        if !self.installs.is_empty() {
            let c = pos2(rect.right() - 16.0, rect.center().y);
            painter.circle_filled(c, 10.0, if selected { Color32::WHITE } else { WINE });
            let n = self.installs.len().to_string();
            painter.text(c, Align2::CENTER_CENTER, n, FontId::new(12.0, demi()), if selected { WINE } else { Color32::WHITE });
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    pub(super) fn discover_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        self.discover_banner(ui);
        egui::Frame::new().inner_margin(Margin { left: 28, right: 28, top: 18, bottom: 28 }).show(ui, |ui| {
            // Buscador y categorías
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("🔍  Buscar apps, juegos o editores…")
                        .desired_width(300.0)
                        .margin(vec2(12.0, 8.0)),
                );
                ui.add_space(6.0);
                let mut chosen = None;
                if chip(ui, "Todo", self.category.is_none()) {
                    chosen = Some(None);
                }
                for c in &self.catalog.categories {
                    if self.catalog.apps.iter().any(|a| a.category == c.id)
                        && chip(ui, &c.name, self.category.as_deref() == Some(c.id.as_str()))
                    {
                        chosen = Some(Some(c.id.clone()));
                    }
                }
                if let Some(c) = chosen {
                    self.category = c;
                }
            });
            ui.add_space(18.0);

            let filtering = !self.search.trim().is_empty() || self.category.is_some();
            if filtering {
                let results: Vec<CatalogApp> =
                    self.catalog.search(&self.search, self.category.as_deref()).into_iter().cloned().collect();
                let title = match &self.category {
                    Some(c) if self.search.trim().is_empty() => self.catalog.category_name(c).to_string(),
                    _ => "Resultados".into(),
                };
                section_title(ui, &title, results.len());
                if results.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Nada por aquí").family(demi()).size(18.0));
                        ui.label(
                            RichText::new("Prueba con otro nombre, o instala cualquier programa con «Ejecutar…» en un bottle.")
                                .weak(),
                        );
                    });
                } else {
                    self.grid(ui, &results, actions);
                }
            } else {
                let featured: Vec<CatalogApp> = self.catalog.apps.iter().filter(|a| a.featured).cloned().collect();
                if !featured.is_empty() {
                    section_title(ui, "Destacadas", 0);
                    self.featured_row(ui, &featured, actions);
                    ui.add_space(26.0);
                }
                for c in self.catalog.categories.clone() {
                    let apps: Vec<CatalogApp> = self.catalog.apps.iter().filter(|a| a.category == c.id).cloned().collect();
                    if apps.is_empty() {
                        continue;
                    }
                    section_title(ui, &c.name, apps.len());
                    self.grid(ui, &apps, actions);
                    ui.add_space(22.0);
                }
            }

            ui.add_space(10.0);
            let updated = if self.catalog.updated.is_empty() { String::new() } else { format!("Catálogo del {} · ", self.catalog.updated) };
            ui.label(
                RichText::new(format!(
                    "{updated}Se actualiza solo. Las versiones vienen de winget-pkgs o del instalador oficial de cada \
                     fabricante; los logos, de Simple Icons. Las marcas son de sus dueños."
                ))
                .small()
                .weak(),
            );
        });
    }

    fn discover_banner(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 196.0), Sense::hover());
        let painter = ui.painter_at(rect);
        gradient(&painter, rect, 0.0, diagonal(rect, Color32::from_rgb(0x9A, 0x1F, 0x48), Color32::from_rgb(0x24, 0x0A, 0x1E)));
        // Logos de las destacadas, flotando a la derecha.
        let featured: Vec<&CatalogApp> = self.catalog.apps.iter().filter(|a| a.featured).take(4).collect();
        let spots = [(vec2(-250.0, 34.0), 64.0), (vec2(-150.0, 92.0), 76.0), (vec2(-118.0, 18.0), 52.0), (vec2(-246.0, 120.0), 46.0)];
        if rect.width() > 640.0 {
            for (app, (offset, size)) in featured.iter().zip(spots) {
                let tile = Rect::from_min_size(pos2(rect.right(), rect.top()) + offset, vec2(size, size));
                shadow(&painter, tile, (size * 0.26) as u8, 70);
                brand_icon(ui, tile, app, self.icons.get(&app.id));
            }
        }
        let left = rect.left() + 28.0;
        painter.text(pos2(left, rect.top() + 64.0), Align2::LEFT_CENTER, "Descubrir", FontId::new(34.0, demi()), Color32::WHITE);
        painter.text(
            pos2(left, rect.top() + 102.0),
            Align2::LEFT_CENTER,
            "Apps y tiendas de Windows, listas para instalar con un clic.",
            FontId::proportional(16.0),
            Color32::from_white_alpha(215),
        );
        let installed = self.found.len();
        let line = if installed > 0 {
            format!("{} apps · {installed} instaladas · cada una en su bottle", self.catalog.apps.len())
        } else {
            format!("{} apps · cada una en su bottle, con los ajustes que necesita", self.catalog.apps.len())
        };
        painter.text(pos2(left, rect.top() + 132.0), Align2::LEFT_CENTER, line, FontId::proportional(13.0), Color32::from_white_alpha(160));
    }

    fn featured_row(&self, ui: &mut egui::Ui, apps: &[CatalogApp], actions: &mut Vec<Action>) {
        let gap = 16.0;
        let width = ui.available_width();
        let cols = (((width + gap) / (300.0 + gap)).floor() as usize).clamp(1, apps.len().max(1));
        let card_w = (width - gap * (cols - 1) as f32) / cols as f32;
        for row in apps.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for app in row {
                    self.featured_card(ui, app, card_w, actions);
                }
            });
            ui.add_space(gap);
        }
    }

    fn featured_card(&self, ui: &mut egui::Ui, app: &CatalogApp, width: f32, actions: &mut Vec<Action>) {
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 186.0), Sense::click());
        let c = hex(&app.color);
        let lift = if resp.hovered() { -3.0 } else { 0.0 };
        let card = rect.translate(vec2(0.0, lift));
        shadow(ui.painter(), card, 18, if resp.hovered() { 90 } else { 45 });
        gradient(ui.painter(), card, 18.0, diagonal(card, mix(c, Color32::WHITE, 0.1), mix(c, Color32::BLACK, 0.45)));
        let fg = on(c);
        // El logo en grande, como marca de agua.
        let mark = Rect::from_center_size(pos2(card.right() - 62.0, card.center().y - 6.0), vec2(118.0, 118.0));
        let faded = if fg == Color32::WHITE { Color32::from_white_alpha(34) } else { Color32::from_black_alpha(34) };
        glyph(ui, mark, app, self.icons.get(&app.id), faded);

        let icon = Rect::from_min_size(card.min + vec2(20.0, 20.0), vec2(48.0, 48.0));
        let glass = if fg == Color32::WHITE { Color32::from_white_alpha(38) } else { Color32::from_black_alpha(28) };
        ui.painter().rect_filled(icon, 13, glass);
        glyph(ui, icon.shrink(10.0), app, self.icons.get(&app.id), fg);
        let painter = ui.painter();
        painter.text(pos2(card.left() + 20.0, card.top() + 94.0), Align2::LEFT_CENTER, &app.name, FontId::new(21.0, demi()), fg);
        let soft = mix(fg, c, 0.25);
        let summary = text_block(ui, &app.summary, FontId::proportional(13.5), soft, card.width() - 150.0, 1);
        ui.painter().galley(pos2(card.left() + 20.0, card.top() + 110.0), summary, soft);

        let id = Id::new(("featured", &app.id));
        let action = Rect::from_min_size(pos2(card.left() + 20.0, card.bottom() - 48.0), vec2(104.0, 30.0));
        let clicked_pill = match self.state(&app.id) {
            State::Installing(_, step) => {
                progress_pill(ui, action.expand2(vec2(8.0, 0.0)), step.fraction(), fg);
                false
            }
            State::Installed(_) => {
                if pill(ui, action, id, "Abrir", glass, fg).clicked() {
                    actions.push(Action::OpenApp(app.id.clone()));
                }
                true
            }
            State::Idle => {
                if pill(ui, action, id, "Instalar", fg, if fg == Color32::WHITE { mix(c, Color32::BLACK, 0.25) } else { Color32::WHITE }).clicked() {
                    actions.push(Action::Install(app.id.clone()));
                }
                true
            }
        };
        let over_pill = clicked_pill && ui.rect_contains_pointer(action);
        if resp.clicked() && !over_pill {
            actions.push(Action::ShowApp(app.id.clone()));
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    fn grid(&self, ui: &mut egui::Ui, apps: &[CatalogApp], actions: &mut Vec<Action>) {
        let gap = 14.0;
        let width = ui.available_width();
        let cols = (((width + gap) / (250.0 + gap)).floor() as usize).max(1);
        let card_w = (width - gap * (cols - 1) as f32) / cols as f32;
        for row in apps.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for app in row {
                    self.card(ui, app, card_w, actions);
                }
            });
            ui.add_space(gap);
        }
    }

    fn card(&self, ui: &mut egui::Ui, app: &CatalogApp, width: f32, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 166.0), Sense::click());
        let hovered = resp.hovered();
        let card = rect.translate(vec2(0.0, if hovered { -2.0 } else { 0.0 }));
        if hovered {
            shadow(ui.painter(), card, 16, 55);
        }
        ui.painter().rect_filled(card, 16, if hovered { p.card_hover } else { p.card });
        let edge = if ui.visuals().dark_mode { Color32::from_white_alpha(10) } else { Color32::from_black_alpha(14) };
        let edge = if hovered { alpha(hex(&app.color), 150) } else { edge };
        ui.painter().rect_stroke(card, 16, Stroke::new(1.0, edge), egui::StrokeKind::Inside);

        let icon = Rect::from_min_size(card.min + vec2(16.0, 16.0), vec2(52.0, 52.0));
        brand_icon(ui, icon, app, self.icons.get(&app.id));
        let text_w = card.right() - icon.right() - 28.0;
        let name = text_block(ui, &app.name, FontId::new(15.5, demi()), p.text, text_w, 1);
        ui.painter().galley(pos2(icon.right() + 12.0, icon.top() + 7.0), name, p.text);
        let by = text_block(ui, &app.publisher, FontId::proportional(12.5), p.muted, text_w, 1);
        ui.painter().galley(pos2(icon.right() + 12.0, icon.top() + 29.0), by, p.muted);
        let summary = text_block(ui, &app.summary, FontId::proportional(13.0), p.muted, card.width() - 32.0, 2);
        ui.painter().galley(pos2(card.left() + 16.0, icon.bottom() + 12.0), summary, p.muted);

        // Abajo: de dónde sale (o en qué va) a la izquierda y la acción a la derecha.
        let action = Rect::from_min_size(pos2(card.right() - 16.0 - 92.0, card.bottom() - 16.0 - 30.0), vec2(92.0, 30.0));
        let foot = pos2(card.left() + 16.0, action.center().y);
        let foot_w = action.left() - foot.x - 10.0;
        let (note, dot) = match self.state(&app.id) {
            State::Installing(_, step) => (step.label(), None),
            State::Installed(bottle) => (format!("En «{bottle}»"), Some(RUNNING)),
            State::Idle if app.winget.is_some() => ("Última versión vía winget".into(), None),
            State::Idle => ("Instalador oficial".into(), None),
        };
        let x = if let Some(color) = dot {
            ui.painter().circle_filled(foot + vec2(4.0, 0.0), 4.0, color);
            foot.x + 14.0
        } else {
            foot.x
        };
        let note = text_block(ui, &note, FontId::proportional(12.0), p.muted, foot_w - (x - foot.x), 1);
        ui.painter().galley(pos2(x, foot.y - note.size().y / 2.0), note, p.muted);

        let id = Id::new(("card", &app.id));
        let interactive = match self.state(&app.id) {
            State::Installing(_, step) => {
                progress_pill(ui, action.shrink2(vec2(0.0, 9.0)), step.fraction(), WINE);
                false
            }
            State::Installed(_) => {
                let fill = if ui.visuals().dark_mode { Color32::from_white_alpha(18) } else { Color32::from_black_alpha(12) };
                if pill(ui, action, id, "Abrir", fill, p.text).clicked() {
                    actions.push(Action::OpenApp(app.id.clone()));
                }
                true
            }
            State::Idle => {
                if pill(ui, action, id, "Instalar", WINE, Color32::WHITE).clicked() {
                    actions.push(Action::Install(app.id.clone()));
                }
                true
            }
        };
        if resp.clicked() && !(interactive && ui.rect_contains_pointer(action)) {
            actions.push(Action::ShowApp(app.id.clone()));
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    /// Ficha de la app: qué es, de dónde sale, cómo se prepara y dónde instalarla.
    pub(super) fn app_detail(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(id) = self.detail.clone() else { return };
        let Some(app) = self.catalog.app(&id).cloned() else {
            self.detail = None;
            return;
        };
        let window = ctx.style_of(ctx.theme()).visuals.window_fill;
        let frame = egui::Frame::new().fill(window).corner_radius(20).inner_margin(Margin::ZERO);
        let modal = egui::Modal::new(Id::new("app-detail")).frame(frame).show(ctx, |ui| {
            ui.set_width(560.0);
            let p = pal(ui);
            let c = hex(&app.color);
            let fg = on(c);

            // Cabecera con el color de la marca
            let (band, _) = ui.allocate_exact_size(vec2(560.0, 138.0), Sense::hover());
            let top = Rect::from_min_max(band.min, pos2(band.right(), band.bottom() + 20.0));
            gradient(ui.painter(), top, 20.0, diagonal(top, mix(c, Color32::WHITE, 0.1), mix(c, Color32::BLACK, 0.45)));
            ui.painter().rect_filled(
                Rect::from_min_max(pos2(band.left(), band.bottom()), pos2(band.right(), band.bottom() + 20.0)),
                0,
                window,
            );
            let faded = if fg == Color32::WHITE { Color32::from_white_alpha(30) } else { Color32::from_black_alpha(30) };
            let mark = Rect::from_center_size(pos2(band.right() - 70.0, band.center().y), vec2(120.0, 120.0));
            glyph(ui, mark, &app, self.icons.get(&app.id), faded);
            let icon = Rect::from_min_size(band.min + vec2(26.0, 30.0), vec2(78.0, 78.0));
            shadow(ui.painter(), icon, 20, 80);
            brand_icon(ui, icon, &app, self.icons.get(&app.id));
            ui.painter().text(pos2(icon.right() + 18.0, icon.top() + 26.0), Align2::LEFT_CENTER, &app.name, FontId::new(27.0, demi()), fg);
            let sub = [app.publisher.as_str(), self.catalog.category_name(&app.category)]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            ui.painter().text(pos2(icon.right() + 18.0, icon.top() + 58.0), Align2::LEFT_CENTER, sub, FontId::proportional(14.0), mix(fg, c, 0.25));
            let close = Rect::from_center_size(pos2(band.right() - 24.0, band.top() + 24.0), vec2(28.0, 28.0));
            let close_resp = ui.interact(close, Id::new("detail-close"), Sense::click());
            ui.painter().circle_filled(close.center(), 14.0, if close_resp.hovered() { Color32::from_black_alpha(90) } else { Color32::from_black_alpha(50) });
            let (c, k) = (close.center(), 4.5);
            for d in [vec2(k, k), vec2(k, -k)] {
                ui.painter().line_segment([c - d, c + d], Stroke::new(1.8, Color32::WHITE));
            }
            if close_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.detail = None;
            }

            egui::Frame::new().inner_margin(Margin { left: 26, right: 26, top: 4, bottom: 24 }).show(ui, |ui| {
                let text = if app.description.is_empty() { &app.summary } else { &app.description };
                ui.label(RichText::new(text).size(15.0).color(p.text));
                if !app.notes.is_empty() {
                    ui.add_space(8.0);
                    let warn = ui.visuals().warn_fg_color;
                    egui::Frame::new()
                        .fill(alpha(warn, 28))
                        .corner_radius(10)
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(&app.notes).small().color(warn));
                        });
                }
                ui.add_space(14.0);
                let well = ui.visuals().faint_bg_color; // la ventana ya es del color de las tarjetas
                egui::Frame::new().fill(well).corner_radius(14).inner_margin(Margin::symmetric(16, 12)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    egui::Grid::new("detail-grid").num_columns(2).spacing([22.0, 9.0]).show(ui, |ui| {
                        let row = |ui: &mut egui::Ui, k: &str, v: String| {
                            ui.label(RichText::new(k).color(p.muted));
                            ui.label(RichText::new(v).color(p.text));
                            ui.end_row();
                        };
                        let version = if app.winget.is_some() { "La más reciente, al instalar" } else { "La última del fabricante" };
                        row(ui, "Versión", version.into());
                        row(ui, "Origen", app.source());
                        let arch = if app.arch == Arch::Win64 { "64 bits" } else { "32 bits" };
                        row(ui, "Windows", format!("{} · {arch}", windows_label(&app.windows)));
                        row(ui, "Gráficos", app.graphics.short().into());
                        if !app.deps.is_empty() {
                            row(ui, "Dependencias", app.deps.join(", "));
                        }
                        if !app.homepage.is_empty() {
                            ui.label(RichText::new("Web").color(p.muted));
                            ui.hyperlink_to(app.homepage.trim_start_matches("https://"), &app.homepage);
                            ui.end_row();
                        }
                    });
                });
                ui.add_space(18.0);
                self.detail_actions(ui, &app, actions);
            });
        });
        if modal.should_close() {
            self.detail = None;
        }
    }

    fn detail_actions(&mut self, ui: &mut egui::Ui, app: &CatalogApp, actions: &mut Vec<Action>) {
        let p = pal(ui);
        match self.state(&app.id) {
            State::Installing(bottle, step) => {
                ui.label(RichText::new(step.label()).family(demi()).color(p.text));
                ui.add_space(4.0);
                let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 12.0), Sense::hover());
                progress_pill(ui, bar, step.fraction(), WINE);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("En «{bottle}». Puedes cerrar esta ficha: la instalación sigue.")).small().color(p.muted),
                );
            }
            State::Installed(bottle) => {
                let bottle = bottle.to_string();
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, RUNNING);
                    ui.label(RichText::new(format!("Instalada en «{bottle}»")).family(demi()).color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if primary(ui, "   Abrir   ", true).clicked() {
                            actions.push(Action::OpenApp(app.id.clone()));
                        }
                        if ui.button("Ver bottle").clicked() {
                            actions.push(Action::GoBottle(bottle.clone()));
                        }
                    });
                });
            }
            State::Idle => {
                ui.label(RichText::new("Instalar en").family(demi()).color(p.text));
                ui.add_space(2.0);
                let label = |t: &Option<String>| match t {
                    None => "Un bottle nuevo (recomendado)".to_string(),
                    Some(n) => n.clone(),
                };
                egui::ComboBox::from_id_salt("install-target").width(300.0).selected_text(label(&self.target_bottle)).show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut self.target_bottle, None, label(&None));
                        for b in &self.bottles {
                            let kind = if b.linked().is_some() { "CrossOver" } else { windows_label(&b.config.windows) };
                            ui.selectable_value(&mut self.target_bottle, Some(b.name.clone()), format!("{} · {kind}", b.name));
                        }
                    },
                );
                ui.add_space(4.0);
                let mut ready = true;
                match &self.target_bottle {
                    None => {
                        let field = egui::TextEdit::singleline(&mut self.target_name)
                            .desired_width(300.0)
                            .hint_text("Nombre del bottle")
                            .background_color(ui.visuals().faint_bg_color)
                            .margin(vec2(10.0, 6.0));
                        ui.add(field);
                        let name = self.target_name.trim();
                        let taken = self.bottles.iter().any(|b| b.name == name);
                        if self.engine.is_none() {
                            ready = false;
                            ui.colored_label(ui.visuals().warn_fg_color, "Necesitas un motor Wine para bottles nuevos: elígelo en «Motor».");
                        } else if taken || cx::check_name(name).is_err() {
                            ready = false;
                            ui.colored_label(ui.visuals().warn_fg_color, "Elige otro nombre para el bottle.");
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Se crea con {} y lo que esta app necesita. Si algo falla, tus otros bottles no se tocan.",
                                    windows_label(&app.windows)
                                ))
                                .small()
                                .color(p.muted),
                            );
                        }
                    }
                    Some(b) => {
                        let linked = self.bottles.iter().any(|x| &x.name == b && x.linked().is_some());
                        let note = if linked {
                            "Se instala con el motor de CrossOver; sus ajustes no cambian."
                        } else {
                            "Se mantienen los ajustes de ese bottle."
                        };
                        ui.label(RichText::new(note).small().color(p.muted));
                    }
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let install = RichText::new("Instalar").color(Color32::WHITE).family(demi()).size(16.0);
                    let button = egui::Button::new(install).fill(WINE).min_size(vec2(150.0, 40.0)).corner_radius(20);
                    if ui.add_enabled(ready, button).clicked() {
                        actions.push(Action::Install(app.id.clone()));
                    }
                    if ui.add(egui::Button::new("Cancelar").min_size(vec2(0.0, 40.0)).corner_radius(20)).clicked() {
                        self.detail = None;
                    }
                });
            }
        }
    }

    /// Aviso flotante abajo a la derecha al terminar una instalación.
    pub(super) fn toast_view(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(t) = &self.toast else { return };
        let ttl = if t.error { 20.0 } else { 9.0 };
        let age = t.at.elapsed().as_secs_f32();
        if age > ttl {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        let app = self.catalog.app(&t.app).cloned();
        let installed = !t.error && self.found.contains_key(&t.app);
        let mut close = false;
        egui::Area::new(Id::new("toast")).anchor(Align2::RIGHT_BOTTOM, vec2(-20.0, -46.0)).order(egui::Order::Foreground).show(ctx, |ui| {
            let p = pal(ui);
            let edge = if t.error { ui.visuals().error_fg_color } else { Color32::TRANSPARENT };
            egui::Frame::new()
                .fill(ui.visuals().window_fill)
                .corner_radius(16)
                .stroke(Stroke::new(1.0, edge))
                .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(90) })
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.set_max_width(380.0);
                    ui.horizontal(|ui| {
                        if let Some(app) = &app {
                            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                            brand_icon(ui, r, app, self.icons.get(&app.id));
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&t.title).family(demi()).size(15.5).color(p.text));
                            ui.label(RichText::new(&t.body).small().color(p.muted));
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if installed && primary(ui, "Abrir", true).clicked() {
                                    actions.push(Action::OpenApp(t.app.clone()));
                                    close = true;
                                }
                                if t.error && ui.button("Ver detalles").clicked() {
                                    actions.push(Action::ShowApp(t.app.clone()));
                                    close = true;
                                }
                                if ui.button("Cerrar").clicked() {
                                    close = true;
                                }
                            });
                        });
                    });
                });
        });
        if close {
            self.toast = None;
        }
    }
}

fn section_title(ui: &mut egui::Ui, title: &str, count: usize) {
    let p = pal(ui);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).family(demi()).size(19.0).color(p.text));
        if count > 0 {
            ui.label(RichText::new(count.to_string()).color(p.muted));
        }
    });
    ui.add_space(6.0);
}

/// Filtro con forma de píldora.
fn chip(ui: &mut egui::Ui, text: &str, on: bool) -> bool {
    let p = pal(ui);
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::new(13.5, demi()), Color32::WHITE);
    let size = vec2(galley.size().x + 28.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = match (on, resp.hovered()) {
        (true, _) => WINE,
        (false, true) => p.card_hover,
        (false, false) => p.card,
    };
    ui.painter().rect_filled(rect, 16, fill);
    let color = if on { Color32::WHITE } else { p.text };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(13.5, demi()), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_and_initials() {
        assert_eq!(hex("#1B2838"), Color32::from_rgb(0x1B, 0x28, 0x38));
        assert_eq!(hex("rojo"), WINE);
        assert_eq!(on(hex("#FCAF17")), Color32::from_rgb(0x1A, 0x17, 0x1C)); // Rockstar: logo oscuro
        assert_eq!(on(hex("#1B2838")), Color32::WHITE);
        assert_eq!(initials("Amazon Games"), "AG");
        assert_eq!(initials("Battle.net"), "B");
        assert_eq!(initials("FastStone Image Viewer"), "FI");
    }
}
