//! Ventana principal: bottles a la izquierda; a la derecha el bottle con su biblioteca o «Descubrir».

mod discover;

use crate::catalog::{self, Catalog, Step};
use crate::core::{self as cx, Arch, Bottle, BottleConfig, Graphics, Program};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Margin, Rect, RichText, Sense, Stroke, TextStyle, pos2,
    vec2,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

pub fn run() -> cx::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Barrica")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([860.0, 560.0])
            // Sin esto eframe pone su icono por defecto en el Dock y tapa el del .app.
            .with_icon(Arc::new(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default())),
        ..Default::default()
    };
    eframe::run_native(
        "Barrica",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            theme(&cc.egui_ctx);
            Ok(Box::new(App::new(cc.egui_ctx.clone())))
        }),
    )
    .map_err(|e| e.to_string())
}

// ---------- Tema: colores del logo y Avenir Next ----------

const WINE: Color32 = Color32::from_rgb(0xB3, 0x26, 0x4F);
const RUNNING: Color32 = Color32::from_rgb(0x3D, 0xD6, 0x8C);

fn demi() -> FontFamily {
    FontFamily::Name("demi".into())
}

/// Colores que no están en `Visuals` (tarjetas, texto secundario).
struct Pal {
    card: Color32,
    card_hover: Color32,
    text: Color32,
    muted: Color32,
}

fn pal(ui: &egui::Ui) -> Pal {
    if ui.visuals().dark_mode {
        Pal {
            card: Color32::from_rgb(0x22, 0x1D, 0x25),
            card_hover: Color32::from_rgb(0x2E, 0x28, 0x33),
            text: Color32::from_rgb(0xEE, 0xE8, 0xF0),
            muted: Color32::from_rgb(0x9E, 0x95, 0xA3),
        }
    } else {
        Pal {
            card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            card_hover: Color32::from_rgb(0xF1, 0xEB, 0xEE),
            text: Color32::from_rgb(0x1E, 0x1A, 0x20),
            muted: Color32::from_rgb(0x6F, 0x66, 0x72),
        }
    }
}

fn theme(ctx: &egui::Context) {
    // Avenir Next del sistema: Regular para el texto, Demi Bold para títulos. Sin fuentes empaquetadas.
    let mut fonts = egui::FontDefinitions::default();
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/Avenir Next.ttc") {
        let face = |index| {
            let mut f = egui::FontData::from_owned(bytes.clone());
            f.index = index;
            Arc::new(f)
        };
        fonts.font_data.insert("avenir".into(), face(7));
        fonts.font_data.insert("avenir-demi".into(), face(2));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "avenir".into());
        fonts.families.insert(demi(), [vec!["avenir-demi".to_string()], fallback].concat());
    } else {
        fonts.families.insert(demi(), fallback);
    }
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, demi())),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(15.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        s.spacing.item_spacing = vec2(10.0, 8.0);
        s.spacing.button_padding = vec2(14.0, 6.0);
        s.spacing.interact_size.y = 30.0;
    });

    for (theme, mut v) in [(egui::Theme::Dark, egui::Visuals::dark()), (egui::Theme::Light, egui::Visuals::light())] {
        let dark = theme == egui::Theme::Dark;
        let rgb = Color32::from_rgb;
        v.panel_fill = if dark { rgb(0x17, 0x14, 0x19) } else { rgb(0xF6, 0xF2, 0xF0) };
        v.window_fill = if dark { rgb(0x1F, 0x1B, 0x22) } else { rgb(0xFF, 0xFF, 0xFF) };
        v.extreme_bg_color = if dark { rgb(0x10, 0x0E, 0x12) } else { rgb(0xFF, 0xFF, 0xFF) };
        v.faint_bg_color = if dark { rgb(0x1F, 0x1B, 0x22) } else { rgb(0xEE, 0xE8, 0xEA) };
        v.selection.bg_fill = WINE;
        v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        v.hyperlink_color = if dark { rgb(0xF2, 0x7C, 0x9E) } else { WINE };
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(10);
        let idle = if dark { rgb(0x2B, 0x26, 0x2F) } else { rgb(0xE9, 0xE2, 0xE5) };
        let hover = if dark { rgb(0x37, 0x31, 0x3C) } else { rgb(0xDF, 0xD6, 0xDA) };
        for (w, fill) in [
            (&mut v.widgets.inactive, idle),
            (&mut v.widgets.hovered, hover),
            (&mut v.widgets.active, WINE),
            (&mut v.widgets.open, hover),
        ] {
            w.corner_radius = CornerRadius::same(8);
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::NONE;
        }
        v.widgets.noninteractive.corner_radius = CornerRadius::same(8);
        // El gris por defecto de egui queda apagado: texto con más contraste.
        let (text, strong) = if dark { (rgb(0xE4, 0xDE, 0xE7), Color32::WHITE) } else { (rgb(0x2A, 0x25, 0x2C), Color32::BLACK) };
        v.widgets.noninteractive.fg_stroke.color = text;
        v.widgets.inactive.fg_stroke.color = text;
        v.widgets.hovered.fg_stroke.color = strong;
        ctx.set_visuals_of(theme, v);
    }
}

/// Botón principal en color vino.
fn primary(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let label = RichText::new(text).color(Color32::WHITE).family(demi());
    ui.add_enabled(enabled, egui::Button::new(label).fill(WINE).min_size(vec2(0.0, 34.0)))
}

/// El barril del logo en pequeño, dibujado con vectores.
fn barrel(painter: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let (w, h) = (rect.width() * 0.66, rect.height() * 0.86);
    let body = Rect::from_center_size(c, vec2(w, h));
    painter.rect_filled(body, CornerRadius::same((w * 0.32) as u8), Color32::from_rgb(0xC8, 0x86, 0x45));
    painter.rect_filled(
        Rect::from_center_size(c - vec2(w * 0.18, 0.0), vec2(w * 0.22, h * 0.9)),
        CornerRadius::same((w * 0.1) as u8),
        Color32::from_rgb(0xDE, 0xA5, 0x62),
    );
    for f in [0.2, 0.8] {
        let y = body.top() + body.height() * f;
        painter.line_segment(
            [pos2(body.left() + 0.5, y), pos2(body.right() - 0.5, y)],
            Stroke::new((h * 0.09).max(1.5), Color32::from_rgb(0x2B, 0x24, 0x27)),
        );
    }
}

/// Degradado vertical (para el banner).
fn vgradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [
        (rect.left_top(), top),
        (rect.right_top(), top),
        (rect.left_bottom(), bottom),
        (rect.right_bottom(), bottom),
    ] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(mesh);
}

fn uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display())
}

/// Color estable por nombre, para los programas sin icono.
fn avatar_color(name: &str) -> Color32 {
    const COLORS: [Color32; 6] = [
        Color32::from_rgb(0xB3, 0x26, 0x4F),
        Color32::from_rgb(0x2F, 0x6F, 0xED),
        Color32::from_rgb(0x1F, 0x9D, 0x7A),
        Color32::from_rgb(0xD9, 0x7A, 0x1C),
        Color32::from_rgb(0x7C, 0x4D, 0xD6),
        Color32::from_rgb(0x4A, 0x55, 0x68),
    ];
    COLORS[name.bytes().map(usize::from).sum::<usize>() % COLORS.len()]
}

// ---------- Estado ----------

const TOOLS: &[(&str, &str)] = &[
    ("winecfg", "Configuración de Wine"),
    ("regedit", "Editor del registro"),
    ("taskmgr", "Administrador de tareas"),
    ("control", "Panel de control"),
    ("explorer", "Explorador"),
    ("uninstaller", "Agregar o quitar programas"),
];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Library,
    Tools,
    Settings,
    Software,
    Snapshots,
    Logs,
    Maintenance,
}

const TABS: [(Tab, &str); 7] = [
    (Tab::Library, "Biblioteca"),
    (Tab::Tools, "Herramientas"),
    (Tab::Settings, "Ajustes"),
    (Tab::Software, "Software"),
    (Tab::Snapshots, "Snapshots"),
    (Tab::Logs, "Registros"),
    (Tab::Maintenance, "Mantenimiento"),
];

enum Msg {
    Done(String),
    Failed(String),
    Info(String),
    Refresh,
    Size(String, u64),
    Installed(String, Vec<(String, String)>),
    Catalog(Catalog),
    Icon(String, PathBuf),
    Step(String, Step),
    InstallDone(String, cx::Result<catalog::Outcome>),
}

enum Action {
    Select(String),
    NewForm,
    Create,
    RunPicked,
    Launch(Program),
    Tool(&'static str),
    OpenC,
    Terminal,
    Kill(bool),
    Save,
    Snapshot,
    Restore(String),
    DeleteSnapshot(String),
    Rename,
    Duplicate,
    Export,
    Trash,
    ImportArchive,
    ImportCrossover(PathBuf),
    LinkCrossover(PathBuf),
    SetEngine(PathBuf),
    PickEngine,
    Open(PathBuf),
    ToggleDefault,
    LoadInstalled,
    Uninstall(String),
    Discover,
    ShowApp(String),
    Install(String),
    OpenApp(String),
    GoBottle(String),
}

struct App {
    ctx: egui::Context,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    bottles: Vec<Bottle>,
    selected: Option<String>,
    settings: cx::Settings,
    engine: Option<PathBuf>,
    engines: Vec<cx::Engine>,
    crossover: Vec<PathBuf>,
    sizes: HashMap<String, u64>,
    running: HashSet<String>,
    polled: Instant,
    tab: Tab,
    // Detalle del bottle seleccionado
    draft: BottleConfig,
    env_text: String,
    programs: Vec<Program>,
    snapshots: Vec<String>,
    logs: Vec<PathBuf>,
    installed: Option<Vec<(String, String)>>,
    snapshot_label: String,
    rename_to: String,
    confirm_trash: bool,
    // Formulario de nuevo bottle
    creating: bool,
    new_name: String,
    new_windows: String,
    new_arch: Arch,
    busy: Option<String>,
    status: String,
    error: bool,
    // Descubrir: el catálogo de instalación
    discover: bool,
    catalog: Catalog,
    icons: HashMap<String, PathBuf>,
    found: BTreeMap<String, (String, PathBuf)>,
    search: String,
    category: Option<String>,
    detail: Option<String>,
    target_bottle: Option<String>,
    target_name: String,
    installs: BTreeMap<String, (String, Step)>,
    toast: Option<Toast>,
}

/// Aviso flotante al terminar una instalación.
struct Toast {
    app: String,
    title: String,
    body: String,
    error: bool,
    at: Instant,
}

impl App {
    fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        let mut app = App {
            ctx,
            tx,
            rx,
            bottles: vec![],
            selected: None,
            settings: cx::Settings::load(),
            engine: None,
            engines: vec![],
            crossover: vec![],
            sizes: HashMap::new(),
            running: HashSet::new(),
            polled: Instant::now() - Duration::from_secs(60),
            tab: Tab::Library,
            draft: BottleConfig::default(),
            env_text: String::new(),
            programs: vec![],
            snapshots: vec![],
            logs: vec![],
            installed: None,
            snapshot_label: String::new(),
            rename_to: String::new(),
            confirm_trash: false,
            creating: false,
            new_name: String::new(),
            new_windows: "win10".into(),
            new_arch: Arch::Win64,
            busy: None,
            status: String::new(),
            error: false,
            discover: false,
            catalog: catalog::load(),
            icons: HashMap::new(),
            found: BTreeMap::new(),
            search: String::new(),
            category: None,
            detail: None,
            target_bottle: None,
            target_name: String::new(),
            installs: BTreeMap::new(),
            toast: None,
        };
        app.selected = app.settings.default_bottle.clone();
        app.refresh();
        app.discover = app.bottles.is_empty(); // la primera vez, directo al catálogo
        app.load_icons();
        // Catálogo nuevo y logos que falten, sin bloquear la ventana.
        let (tx, ctx, current) = (app.tx.clone(), app.ctx.clone(), app.catalog.clone());
        std::thread::spawn(move || {
            let cat = match catalog::refresh() {
                Ok(c) => {
                    let _ = tx.send(Msg::Catalog(c.clone()));
                    c
                }
                Err(_) => current,
            };
            for a in cat.apps.iter().filter(|a| catalog::icon(a).is_none()) {
                if let Ok(p) = catalog::fetch_icon(a) {
                    let _ = tx.send(Msg::Icon(a.id.clone(), p));
                }
            }
            ctx.request_repaint();
        });
        app
    }

    fn load_icons(&mut self) {
        self.icons = self.catalog.apps.iter().filter_map(|a| Some((a.id.clone(), catalog::icon(a)?))).collect();
    }

    fn current(&self) -> Option<&Bottle> {
        self.selected.as_ref().and_then(|n| self.bottles.iter().find(|b| &b.name == n))
    }

    /// Relee todo del disco y recalcula tamaños en segundo plano (`du` tarda con bottles grandes).
    fn refresh(&mut self) {
        self.bottles = cx::list();
        self.settings = cx::Settings::load();
        self.engines = cx::engines();
        self.engine = cx::engine_for(None).ok();
        self.crossover = cx::crossover_bottles();
        self.found = catalog::installed(&self.catalog, &self.bottles);
        if self.current().is_none() {
            self.selected = self.bottles.first().map(|b| b.name.clone());
        }
        if let Some(name) = self.selected.clone() {
            self.select(&name);
        }
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        let paths: Vec<(String, PathBuf)> = self.bottles.iter().map(|b| (b.name.clone(), b.path.clone())).collect();
        std::thread::spawn(move || {
            for (name, path) in paths {
                if let Some(size) = cx::size(&path) {
                    let _ = tx.send(Msg::Size(name, size));
                    ctx.request_repaint();
                }
            }
        });
    }

    fn select(&mut self, name: &str) {
        let Ok(b) = cx::get(name) else { return };
        if self.selected.as_deref() != Some(name) {
            self.installed = None;
            self.confirm_trash = false;
            self.tab = Tab::Library;
        }
        self.selected = Some(name.into());
        self.env_text = b.config.env.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
        self.draft = b.config.clone();
        self.programs = cx::programs(&b);
        // Apps del catálogo instaladas sin acceso directo en el menú Inicio: también a la biblioteca.
        for app in &self.catalog.apps {
            if let Some(exe) = catalog::find_in(app, &b)
                && !self.programs.iter().any(|p| p.path == exe || p.name.eq_ignore_ascii_case(&app.name))
            {
                self.programs.push(Program { name: app.name.clone(), path: exe, args: vec![], icon: None, cover: None, hero: None });
            }
        }
        self.snapshots = cx::snapshots(&b);
        self.logs = cx::logs(&b).into_iter().take(10).collect();
        self.rename_to = b.name;
    }

    fn ok(&mut self, s: impl Into<String>) {
        (self.status, self.error) = (s.into(), false);
    }
    fn fail(&mut self, e: impl Into<String>) {
        (self.status, self.error) = (e.into(), true);
    }

    /// Trabajo largo en otro hilo; la ventana sigue respondiendo.
    fn job(&mut self, label: String, f: impl FnOnce() -> cx::Result<String> + Send + 'static) {
        self.busy = Some(label);
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(match f() {
                Ok(s) => Msg::Done(s),
                Err(e) => Msg::Failed(e),
            });
            ctx.request_repaint();
        });
    }

    /// Espera al proceso en otro hilo (sin zombis). Solo avisa si falló: con Steam, `steam.exe`
    /// termina enseguida aunque el juego siga abierto; para eso está el indicador «En ejecución».
    fn watch(&self, mut child: Child, label: String, log: PathBuf) {
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let msg = match child.wait().ok().and_then(|s| s.code()) {
                Some(0) | None => Msg::Refresh,
                Some(c) => Msg::Info(format!("«{label}» terminó con código {c}. Log: {}", log.display())),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn drain(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Done(s) => {
                    self.busy = None;
                    self.ok(s);
                    self.refresh();
                }
                Msg::Failed(e) => {
                    self.busy = None;
                    self.fail(e);
                    self.refresh();
                }
                Msg::Info(s) => {
                    self.fail(s);
                    self.reselect();
                }
                Msg::Refresh => self.reselect(),
                Msg::Size(name, size) => {
                    self.sizes.insert(name, size);
                }
                Msg::Installed(name, list) => {
                    self.busy = None;
                    if self.selected.as_deref() == Some(&name) {
                        self.installed = Some(list);
                    }
                }
                Msg::Catalog(c) => {
                    self.catalog = c;
                    self.load_icons();
                    self.found = catalog::installed(&self.catalog, &self.bottles);
                }
                Msg::Icon(id, path) => {
                    self.icons.insert(id, path);
                }
                Msg::Step(id, step) => {
                    if let Some(entry) = self.installs.get_mut(&id) {
                        entry.1 = step;
                    }
                }
                Msg::InstallDone(id, result) => self.install_done(id, result),
            }
        }
        // ¿Qué bottles tienen un wineserver vivo? Barato: un `stat` por bottle cada 2 s.
        if self.polled.elapsed() > Duration::from_secs(2) {
            self.running = self.bottles.iter().filter(|b| cx::is_running(b)).map(|b| b.name.clone()).collect();
            self.polled = Instant::now();
        }
        self.ctx.request_repaint_after(Duration::from_secs(2));
    }

    /// Un instalador pudo crear accesos directos: relee el bottle actual.
    fn reselect(&mut self) {
        if let Some(name) = self.selected.clone() {
            self.select(&name);
        }
    }

    fn handle(&mut self, action: Action) {
        let Some(b) = self.current().cloned() else {
            return self.handle_global(action);
        };
        let name = b.name.clone();
        match action {
            Action::RunPicked => {
                if let Some(path) = choose_file("Elige un programa o instalador (.exe, .msi, .lnk, .bat)") {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    let p = Program { name, path, args: vec![], icon: None, cover: None, hero: None };
                    self.handle(Action::Launch(p));
                }
            }
            Action::Launch(p) => match cx::launch(&b, &p.path, &p.args, None) {
                Ok((child, log)) => {
                    self.ok(format!("Abriendo «{}»…", p.name));
                    self.polled = Instant::now() - Duration::from_secs(60);
                    self.watch(child, p.name, log);
                }
                Err(e) => self.fail(e),
            },
            Action::Tool(tool) => match cx::tool(&b, tool) {
                Ok((child, log)) => self.watch(child, tool.into(), log),
                Err(e) => self.fail(e),
            },
            Action::OpenC => report(self, cx::open(&b.drive_c())),
            Action::Terminal => report(self, cx::terminal(&b)),
            Action::Kill(force) => {
                self.job(format!("Cerrando «{name}»…"), move || cx::kill(&b, force).map(|_| "Programas cerrados.".into()))
            }
            Action::Save => {
                let mut nb = b.clone();
                nb.config = self.draft.clone();
                nb.config.env = parse_env(&self.env_text);
                let registry =
                    nb.config.windows != b.config.windows || nb.config.high_resolution != b.config.high_resolution;
                if let Err(e) = nb.save() {
                    return self.fail(e);
                }
                if registry {
                    self.job("Aplicando ajustes de Windows…".into(), move || {
                        cx::apply_settings(&nb).map(|_| "Ajustes guardados.".into())
                    });
                } else {
                    self.ok("Ajustes guardados.");
                    self.refresh();
                }
            }
            Action::Snapshot => {
                let label = self.snapshot_label.trim().to_string();
                self.snapshot_label.clear();
                self.job(format!("Creando snapshot «{label}»…"), move || {
                    cx::snapshot(&b, &label).map(|_| format!("Snapshot «{label}» creado."))
                });
            }
            Action::Restore(label) => self.job(format!("Restaurando «{label}»…"), move || {
                cx::restore(&b, &label).map(|_| format!("Restaurado «{label}». El estado anterior está en la Papelera."))
            }),
            Action::DeleteSnapshot(label) => {
                report(self, cx::delete_snapshot(&b, &label));
                self.refresh();
            }
            Action::Rename => match cx::rename(&b, self.rename_to.trim()) {
                Ok(nb) => {
                    self.selected = Some(nb.name);
                    self.ok("Renombrado.");
                    self.refresh();
                }
                Err(e) => self.fail(e),
            },
            Action::Duplicate => {
                let to = unique(&self.bottles, &format!("{name} copia"));
                self.selected = Some(to.clone());
                self.job(format!("Duplicando «{name}»…"), move || {
                    cx::duplicate(&b, &to).map(|_| format!("Duplicado como «{to}» (clon APFS, sin gastar disco)."))
                });
            }
            Action::Export => {
                if let Some(dest) = choose_save(&format!("{name}.tar.gz")) {
                    self.job(format!("Exportando «{name}»…"), move || {
                        cx::export(&b, &dest).map(|_| format!("Exportado a {}", dest.display()))
                    });
                }
            }
            Action::Trash => {
                self.selected = None;
                self.job(format!("Quitando «{name}»…"), move || {
                    let done = if b.linked().is_some() { "se quitó de Barrica" } else { "está en la Papelera" };
                    cx::trash(&b).map(|_| format!("«{}» {done}.", b.name))
                });
            }
            Action::ToggleDefault => {
                let is_default = self.settings.default_bottle.as_deref() == Some(&name);
                self.settings.default_bottle = (!is_default).then_some(name);
                report(self, self.settings.save());
            }
            Action::LoadInstalled => {
                self.busy = Some("Leyendo software instalado…".into());
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || {
                    let _ = tx.send(match cx::installed(&b) {
                        Ok(list) => Msg::Installed(b.name, list),
                        Err(e) => Msg::Failed(e),
                    });
                    ctx.request_repaint();
                });
            }
            Action::Uninstall(id) => match cx::uninstall(&b, &id) {
                Ok((child, log)) => {
                    self.installed = None;
                    self.watch(child, "Desinstalador".into(), log);
                }
                Err(e) => self.fail(e),
            },
            other => self.handle_global(other),
        }
    }

    /// Acciones que no necesitan un bottle seleccionado.
    fn handle_global(&mut self, action: Action) {
        match action {
            Action::Select(n) => {
                self.creating = false;
                self.discover = false;
                self.select(&n);
            }
            Action::NewForm => {
                self.creating = true;
                self.discover = false;
                self.new_name = unique(&self.bottles, "Mi bottle");
            }
            Action::Create => {
                let (n, w, arch) = (self.new_name.trim().to_string(), self.new_windows.clone(), self.new_arch);
                self.creating = false;
                self.selected = Some(n.clone());
                self.job(format!("Creando «{n}»… (Wine prepara Windows, tarda un poco)"), move || {
                    cx::create(&n, &w, arch).map(|_| format!("Bottle «{n}» listo. Instala algo con «Ejecutar…»."))
                });
            }
            Action::ImportArchive => {
                if let Some(p) = choose_file("Elige un bottle exportado (.tar.gz) o un .cxarchive") {
                    self.job("Importando…".into(), move || {
                        cx::import_archive(&p).map(|b| format!("Importado como «{}».", b.name))
                    });
                }
            }
            Action::ImportCrossover(src) => {
                let base = src.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let to = unique(&self.bottles, &base);
                self.selected = Some(to.clone());
                self.job(format!("Copiando «{base}» de CrossOver…"), move || {
                    cx::import_crossover(&src, &to)
                        .map(|_| format!("Copiado como «{to}». El bottle original de CrossOver no se tocó."))
                });
            }
            Action::LinkCrossover(src) => {
                let cx_name = src.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let to = unique(&self.bottles, &cx_name);
                match cx::link_crossover(&cx_name, &to) {
                    Ok(_) => {
                        self.creating = false;
                        self.selected = Some(to.clone());
                        self.ok(format!("«{to}» vinculado: se abre con el motor de CrossOver."));
                        self.refresh();
                    }
                    Err(e) => self.fail(e),
                }
            }
            Action::SetEngine(path) => {
                self.settings.engine = Some(path);
                report(self, self.settings.save());
                self.refresh();
            }
            Action::PickEngine => {
                if let Some(p) = choose_file("Elige el ejecutable wine (o wine64) de tu motor") {
                    self.handle_global(Action::SetEngine(p));
                }
            }
            Action::Open(p) => report(self, cx::open(&p)),
            Action::Discover => {
                self.discover = true;
                self.creating = false;
            }
            Action::ShowApp(id) => self.show_app(id),
            Action::Install(id) => self.install(id),
            Action::OpenApp(id) => self.open_app(&id),
            Action::GoBottle(name) => {
                self.discover = false;
                self.creating = false;
                self.detail = None;
                self.select(&name);
            }
            _ => {}
        }
    }

    // ---------- Vistas ----------

    fn top_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
            barrel(ui.painter(), rect);
            ui.label(RichText::new("Barrica").family(demi()).size(19.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("Motor", |ui| {
                    ui.label(RichText::new("Motor propio para bottles nuevos").small().weak());
                    if self.engines.is_empty() {
                        ui.label("Ninguno instalado (descárgalo de wiki.winehq.org/MacOS).");
                    }
                    for e in &self.engines {
                        if ui.radio(Some(&e.wine) == self.engine.as_ref(), e.label.as_str()).clicked() {
                            actions.push(Action::SetEngine(e.wine.clone()));
                        }
                    }
                    if ui.button("Elegir ejecutable de wine…").clicked() {
                        actions.push(Action::PickEngine);
                    }
                    ui.separator();
                    ui.label(RichText::new("Los bottles vinculados usan el motor de CrossOver.").small().weak());
                });
                ui.menu_button("Importar", |ui| {
                    ui.label(RichText::new("De CrossOver").small().weak());
                    if self.crossover.is_empty() {
                        ui.label("No hay bottles de CrossOver.");
                    }
                    for p in &self.crossover {
                        let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                        ui.menu_button(name, |ui| {
                            if ui.button("Vincular: usa el motor de CrossOver (para juegos como CS2)").clicked() {
                                actions.push(Action::LinkCrossover(p.clone()));
                            }
                            if ui.button("Copiar: bottle independiente con motor propio").clicked() {
                                actions.push(Action::ImportCrossover(p.clone()));
                            }
                        });
                    }
                    ui.separator();
                    if ui.button("Desde un archivo…").clicked() {
                        actions.push(Action::ImportArchive);
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = pal(ui);
        ui.add_space(12.0);
        if self.discover_nav(ui) {
            actions.push(Action::Discover);
        }
        ui.add_space(14.0);
        ui.label(RichText::new("BOTTLES").size(11.5).color(p.muted).family(demi()));
        ui.add_space(4.0);
        let list_height = (ui.available_height() - 60.0).max(80.0);
        if self.bottles.is_empty() {
            ui.label(RichText::new("Todavía ninguno. Instala una app y Barrica le crea el suyo.").small().color(p.muted));
        }
        egui::ScrollArea::vertical().auto_shrink([false, true]).max_height(list_height).show(ui, |ui| {
            for b in &self.bottles {
                let selected = !self.creating && !self.discover && self.selected.as_deref() == Some(&b.name);
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
                let fill = if selected {
                    WINE
                } else if resp.hovered() {
                    p.card_hover
                } else {
                    Color32::TRANSPARENT
                };
                let painter = ui.painter();
                painter.rect_filled(rect, 10, fill);
                let icon = Rect::from_min_size(rect.min + vec2(8.0, 10.0), vec2(34.0, 34.0));
                match self.bottle_app(b) {
                    Some(app) => discover::brand_icon(ui, icon.shrink(1.0), app, self.icons.get(&app.id)),
                    None => barrel(painter, icon),
                }
                let (main, sub) =
                    if selected { (Color32::WHITE, Color32::from_white_alpha(190)) } else { (p.text, p.muted) };
                let star = if self.settings.default_bottle.as_deref() == Some(&b.name) { " ★" } else { "" };
                painter.text(
                    pos2(rect.left() + 50.0, rect.top() + 18.0),
                    Align2::LEFT_CENTER,
                    format!("{}{star}", short(&b.name, 22)),
                    FontId::new(15.0, demi()),
                    main,
                );
                let kind = if b.linked().is_some() { "CrossOver" } else { windows_label(&b.config.windows) };
                let size = self.sizes.get(&b.name).map_or_else(|| "…".into(), |s| cx::human(*s));
                painter.text(
                    pos2(rect.left() + 50.0, rect.top() + 37.0),
                    Align2::LEFT_CENTER,
                    format!("{kind} · {size}"),
                    FontId::proportional(12.5),
                    sub,
                );
                if self.running.contains(&b.name) {
                    painter.circle_filled(pos2(rect.right() - 14.0, rect.center().y), 4.5, RUNNING);
                }
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    actions.push(Action::Select(b.name.clone()));
                }
            }
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            if ui.add_sized([ui.available_width(), 36.0], egui::Button::new("+  Nuevo bottle")).clicked() {
                actions.push(Action::NewForm);
            }
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(label) = &self.busy {
                ui.spinner();
                ui.label(RichText::new(label).small());
            } else if let Some((id, (_, step))) = self.installs.iter().next() {
                ui.spinner();
                let name = self.catalog.app(id).map_or(id.as_str(), |a| a.name.as_str());
                let more = if self.installs.len() > 1 { format!(" (+{} más)", self.installs.len() - 1) } else { String::new() };
                ui.label(RichText::new(format!("{name}: {}{more}", step.label())).small());
            } else if !self.status.is_empty() {
                let text = RichText::new(&self.status).small();
                ui.label(if self.error { text.color(ui.visuals().error_fg_color) } else { text.weak() });
            }
        });
    }

    fn empty_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.vertical_centered(|ui| {
            ui.add_space(110.0);
            let (rect, _) = ui.allocate_exact_size(vec2(84.0, 84.0), Sense::hover());
            barrel(ui.painter(), rect);
            ui.add_space(10.0);
            ui.heading("Tu bodega está vacía");
            ui.label(RichText::new("Un bottle es un Windows aparte donde instalas programas y juegos.").weak());
            ui.add_space(14.0);
            if primary(ui, "Explorar apps para instalar", true).clicked() {
                actions.push(Action::Discover);
            }
            if ui.button("Crear un bottle vacío").clicked() {
                actions.push(Action::NewForm);
            }
            if !self.crossover.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("¿Ya usas CrossOver? Tráelo desde «Importar», arriba a la derecha.").weak());
            }
        });
    }

    fn new_bottle_form(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let p = pal(ui);
        egui::Frame::new().inner_margin(Margin::same(28)).show(ui, |ui| {
            ui.heading("Nuevo bottle");
            ui.label(RichText::new("Un Windows limpio para instalar tus programas.").weak());
            ui.add_space(16.0);
            egui::Frame::new().fill(p.card).corner_radius(14).inner_margin(Margin::same(20)).show(ui, |ui| {
                egui::Grid::new("new").num_columns(2).spacing([16.0, 14.0]).show(ui, |ui| {
                    ui.label("Nombre");
                    ui.add(egui::TextEdit::singleline(&mut self.new_name).desired_width(280.0));
                    ui.end_row();
                    ui.label("Windows");
                    windows_combo(ui, "new_windows", &mut self.new_windows);
                    ui.end_row();
                    ui.label("Arquitectura");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.new_arch, Arch::Win64, "64 bits");
                        ui.radio_value(&mut self.new_arch, Arch::Win32, "32 bits (programas antiguos)");
                    });
                    ui.end_row();
                });
            });
            ui.add_space(14.0);
            let exists = self.bottles.iter().any(|b| b.name == self.new_name.trim());
            let valid = cx::check_name(self.new_name.trim()).is_ok() && !exists;
            ui.horizontal(|ui| {
                if primary(ui, "Crear bottle", valid && self.busy.is_none() && self.engine.is_some()).clicked() {
                    actions.push(Action::Create);
                }
                if ui.button("Cancelar").clicked() {
                    self.creating = false;
                }
            });
            if exists {
                ui.colored_label(ui.visuals().warn_fg_color, "Ya hay un bottle con ese nombre.");
            }
            if self.engine.is_none() {
                ui.colored_label(ui.visuals().warn_fg_color, "Necesitas un motor propio: elígelo en «Motor», arriba.");
            }
            ui.label(RichText::new("Wine prepara el Windows nuevo: tarda entre 10 segundos y un minuto.").small().weak());
        });
    }

    /// Banner: el arte del juego (si hay) con el nombre del bottle encima.
    fn hero(&self, ui: &mut egui::Ui, b: &Bottle, running: bool) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, 210.0), Sense::hover());
        let painter = ui.painter_at(rect);
        match self.programs.iter().find_map(|p| p.hero.as_ref()) {
            Some(hero) => {
                // Recorte tipo «cover» del banner de Steam (1920×620).
                let (aspect, ratio) = (1920.0 / 620.0, rect.width() / rect.height());
                let uv = if ratio > aspect {
                    let h = aspect / ratio;
                    Rect::from_min_max(pos2(0.0, 0.5 - h / 2.0), pos2(1.0, 0.5 + h / 2.0))
                } else {
                    let w = ratio / aspect;
                    Rect::from_min_max(pos2(0.5 - w / 2.0, 0.0), pos2(0.5 + w / 2.0, 1.0))
                };
                egui::Image::new(uri(hero)).uv(uv).paint_at(ui, rect);
            }
            // Un bottle creado para una app del catálogo se viste con su marca.
            None => match self.bottle_app(b) {
                Some(app) => discover::brand_banner(ui, rect, app, self.icons.get(&app.id)),
                None => vgradient(&painter, rect, Color32::from_rgb(0x8E, 0x1B, 0x3F), Color32::from_rgb(0x2A, 0x08, 0x14)),
            },
        }
        let fade = Rect::from_min_max(pos2(rect.left(), rect.center().y - 30.0), rect.max);
        vgradient(&painter, fade, Color32::TRANSPARENT, Color32::from_black_alpha(215));

        let arch = if b.config.arch == Arch::Win64 { "64 bits" } else { "32 bits" };
        let mut subtitle = format!("{} · {arch} · {}", windows_label(&b.config.windows), b.config.graphics.short());
        if b.linked().is_some() {
            subtitle = format!("Motor de CrossOver · {subtitle}");
        }
        if !b.config.description.is_empty() && b.config.description != b.name {
            subtitle = format!("{} · {subtitle}", b.config.description);
        }
        let left = rect.left() + 28.0;
        painter.text(
            pos2(left, rect.bottom() - 54.0),
            Align2::LEFT_CENTER,
            &b.name,
            FontId::new(32.0, demi()),
            Color32::WHITE,
        );
        painter.text(
            pos2(left, rect.bottom() - 24.0),
            Align2::LEFT_CENTER,
            subtitle,
            FontId::proportional(14.0),
            Color32::from_white_alpha(200),
        );
        if running {
            let pill = Rect::from_min_size(pos2(rect.right() - 160.0, rect.top() + 18.0), vec2(140.0, 30.0));
            painter.rect_filled(pill, 15, Color32::from_black_alpha(150));
            painter.circle_filled(pos2(pill.left() + 18.0, pill.center().y), 5.0, RUNNING);
            painter.text(
                pos2(pill.left() + 32.0, pill.center().y),
                Align2::LEFT_CENTER,
                "En ejecución",
                FontId::new(14.0, demi()),
                Color32::WHITE,
            );
        }
    }

    fn bottle_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let Some(b) = self.current().cloned() else { return };
        let running = self.running.contains(&b.name);
        self.hero(ui, &b, running);

        egui::Frame::new().inner_margin(Margin { left: 28, right: 28, top: 16, bottom: 28 }).show(ui, |ui| {
            ui.horizontal(|ui| {
                if primary(ui, "Ejecutar…", true).on_hover_text("Un .exe, .msi, .lnk o .bat").clicked() {
                    actions.push(Action::RunPicked);
                }
                if ui.button("Abrir disco C:").clicked() {
                    actions.push(Action::OpenC);
                }
                if ui.button("Terminal").clicked() {
                    actions.push(Action::Terminal);
                }
                if running && ui.button("Cerrar todo").clicked() {
                    actions.push(Action::Kill(false));
                }
                ui.menu_button("Más", |ui| {
                    let is_default = self.settings.default_bottle.as_deref() == Some(&b.name);
                    let label = if is_default { "Quitar como predeterminado" } else { "Abrir al iniciar Barrica" };
                    if ui.button(label).clicked() {
                        actions.push(Action::ToggleDefault);
                    }
                    if ui.button("Mostrar en Finder").clicked() {
                        actions.push(Action::Open(b.path.clone()));
                    }
                    ui.separator();
                    if ui.button("Cerrar todo").clicked() {
                        actions.push(Action::Kill(false));
                    }
                    if ui.button("Forzar cierre").clicked() {
                        actions.push(Action::Kill(true));
                    }
                });
            });
            ui.add_space(14.0);

            ui.horizontal(|ui| {
                let p = pal(ui);
                for (tab, label) in TABS {
                    let on = self.tab == tab;
                    let text = RichText::new(label).family(demi()).color(if on { Color32::WHITE } else { p.muted });
                    if ui.add(egui::Button::selectable(on, text)).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.add_space(14.0);

            match self.tab {
                Tab::Library => self.library(ui, actions),
                Tab::Tools => {
                    ui.horizontal_wrapped(|ui| {
                        for (tool, label) in TOOLS {
                            if ui.button(*label).clicked() {
                                actions.push(Action::Tool(tool));
                            }
                        }
                    });
                }
                Tab::Settings => {
                    if b.linked().is_some() {
                        ui.label(
                            "Este bottle usa el motor de CrossOver: gráficos, MSync y versión de Windows se cambian allí.",
                        );
                        if ui.button("Abrir CrossOver").clicked() {
                            actions.push(Action::Open(cx::crossover_app()));
                        }
                    } else {
                        self.settings_view(ui, &b, actions);
                    }
                }
                Tab::Software => self.software_view(ui, actions),
                Tab::Snapshots => self.snapshots_view(ui, actions),
                Tab::Logs => {
                    ui.label(RichText::new("Cada ejecución guarda su salida: útil cuando algo no abre.").weak());
                    for log in &self.logs {
                        let name = log.file_name().unwrap_or_default().to_string_lossy().into_owned();
                        if ui.link(name).clicked() {
                            actions.push(Action::Open(log.clone()));
                        }
                    }
                    if ui.button("Abrir carpeta de registros").clicked() {
                        actions.push(Action::Open(cx::logs_dir(&b.name)));
                    }
                }
                Tab::Maintenance => self.maintenance_view(ui, &b, actions),
            }
        });
    }

    fn library(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        if self.programs.is_empty() {
            ui.label(RichText::new("Aún no hay programas en este bottle.").weak());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if primary(ui, "Instalar desde Descubrir", true).clicked() {
                    actions.push(Action::Discover);
                }
                if ui.button("Ejecutar un instalador…").clicked() {
                    actions.push(Action::RunPicked);
                }
            });
            return;
        }
        let (games, apps): (Vec<&Program>, Vec<&Program>) = self.programs.iter().partition(|p| p.cover.is_some());
        if !games.is_empty() {
            ui.label(RichText::new("Juegos").family(demi()).size(18.0).color(pal(ui).text));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(18.0, 18.0);
                for g in games {
                    if game_card(ui, g).clicked() {
                        actions.push(Action::Launch(g.clone()));
                    }
                }
            });
            ui.add_space(18.0);
        }
        if !apps.is_empty() {
            ui.label(RichText::new("Aplicaciones").family(demi()).size(18.0).color(pal(ui).text));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                for a in apps {
                    let brand = self.brand_for(a).map(|b| (b, self.icons.get(&b.id)));
                    if app_tile(ui, a, brand).clicked() {
                        actions.push(Action::Launch(a.clone()));
                    }
                }
            });
        }
    }

    fn software_view(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        match &self.installed {
            None => {
                if ui.add_enabled(self.busy.is_none(), egui::Button::new("Ver software instalado")).clicked() {
                    actions.push(Action::LoadInstalled);
                }
            }
            Some(list) if list.is_empty() => {
                ui.label("No hay nada instalado.");
            }
            Some(list) => {
                let p = pal(ui);
                egui::Frame::new().fill(p.card).corner_radius(12).inner_margin(Margin::same(12)).show(ui, |ui| {
                    for (id, name) in list {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Desinstalar").clicked() {
                                    actions.push(Action::Uninstall(id.clone()));
                                }
                            });
                        });
                    }
                });
            }
        }
    }

    fn snapshots_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.label(
            RichText::new(
                "Copias instantáneas (clon APFS): no ocupan espacio hasta que el bottle cambia. \
                 Haz una antes de instalar algo arriesgado, con los programas cerrados.",
            )
            .weak(),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.snapshot_label)
                    .hint_text("p. ej. antes de los mods")
                    .desired_width(240.0),
            );
            let ok = cx::check_name(self.snapshot_label.trim()).is_ok() && self.busy.is_none();
            if primary(ui, "Crear snapshot", ok).clicked() {
                actions.push(Action::Snapshot);
            }
        });
        ui.add_space(8.0);
        let p = pal(ui);
        for s in &self.snapshots {
            egui::Frame::new().fill(p.card).corner_radius(10).inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(s).family(demi()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Borrar").clicked() {
                            actions.push(Action::DeleteSnapshot(s.clone()));
                        }
                        if ui.small_button("Restaurar").clicked() {
                            actions.push(Action::Restore(s.clone()));
                        }
                    });
                });
            });
        }
    }

    fn maintenance_view(&mut self, ui: &mut egui::Ui, b: &Bottle, actions: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.rename_to).desired_width(240.0));
            let valid = cx::check_name(self.rename_to.trim()).is_ok() && self.rename_to.trim() != b.name;
            if ui.add_enabled(valid, egui::Button::new("Renombrar")).clicked() {
                actions.push(Action::Rename);
            }
        });
        ui.horizontal(|ui| {
            if b.linked().is_none() && ui.button("Duplicar").clicked() {
                actions.push(Action::Duplicate);
            }
            if ui.button("Exportar a archivo…").clicked() {
                actions.push(Action::Export);
            }
        });
        ui.add_space(12.0);
        let linked = b.linked().is_some();
        ui.horizontal(|ui| {
            if !self.confirm_trash {
                let label = if linked { "Quitar de Barrica…" } else { "Mover a la Papelera…" };
                if ui.button(RichText::new(label).color(ui.visuals().error_fg_color)).clicked() {
                    self.confirm_trash = true;
                }
            } else {
                ui.label(if linked {
                    format!("¿Quitar «{}» de Barrica? El bottle de CrossOver no se toca.", b.name)
                } else {
                    format!("¿Mover «{}» a la Papelera?", b.name)
                });
                if ui.button("Sí").clicked() {
                    actions.push(Action::Trash);
                }
                if ui.button("No").clicked() {
                    self.confirm_trash = false;
                }
            }
        });
    }

    fn settings_view(&mut self, ui: &mut egui::Ui, b: &Bottle, actions: &mut Vec<Action>) {
        let p = pal(ui);
        egui::Frame::new().fill(p.card).corner_radius(14).inner_margin(Margin::same(18)).show(ui, |ui| {
            egui::Grid::new("settings").num_columns(2).spacing([16.0, 12.0]).show(ui, |ui| {
                ui.label("Windows");
                windows_combo(ui, "windows", &mut self.draft.windows);
                ui.end_row();
                ui.label("Gráficos");
                egui::ComboBox::from_id_salt("graphics").selected_text(self.draft.graphics.label()).show_ui(ui, |ui| {
                    for g in Graphics::ALL {
                        ui.selectable_value(&mut self.draft.graphics, g, g.label());
                    }
                });
                ui.end_row();
                ui.label("Rendimiento");
                ui.checkbox(&mut self.draft.msync, "MSync / ESync (si el motor lo soporta)");
                ui.end_row();
                ui.label("Pantalla");
                ui.checkbox(&mut self.draft.high_resolution, "Alta resolución (Retina)");
                ui.end_row();
                ui.label("Descripción");
                ui.add(egui::TextEdit::singleline(&mut self.draft.description).desired_width(320.0));
                ui.end_row();
                ui.label("Variables");
                ui.add(
                    egui::TextEdit::multiline(&mut self.env_text)
                        .hint_text("CLAVE=valor, una por línea (p. ej. DXVK_HUD=fps)")
                        .desired_rows(3)
                        .desired_width(320.0)
                        .code_editor(),
                );
                ui.end_row();
            });
        });
        ui.add_space(10.0);
        let mut edited = self.draft.clone();
        edited.env = parse_env(&self.env_text);
        if primary(ui, "Guardar ajustes", edited != b.config && self.busy.is_none()).clicked() {
            actions.push(Action::Save);
        }
    }
}

/// Portada vertical estilo biblioteca de Steam; al pasar el cursor aparece «Jugar».
fn game_card(ui: &mut egui::Ui, g: &Program) -> egui::Response {
    let size = vec2(150.0, 225.0);
    let (rect, resp) = ui.allocate_exact_size(size + vec2(0.0, 30.0), Sense::click());
    let lift = if resp.hovered() { vec2(0.0, -3.0) } else { vec2(0.0, 0.0) };
    let cover = Rect::from_min_size(rect.min, size).translate(lift);
    if let Some(path) = &g.cover {
        egui::Image::new(uri(path)).corner_radius(12).paint_at(ui, cover);
    }
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(cover, 12, Color32::from_black_alpha(110));
        painter.circle_filled(cover.center(), 28.0, WINE);
        painter.text(cover.center() + vec2(2.0, 0.0), Align2::CENTER_CENTER, "▶", FontId::proportional(24.0), Color32::WHITE);
    }
    painter.text(
        pos2(rect.left() + 2.0, rect.bottom() - 12.0),
        Align2::LEFT_CENTER,
        short(&g.name, 19),
        FontId::new(14.5, demi()),
        pal(ui).text,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Jugar a {}", g.name))
}

/// Tarjeta de aplicación: el icono real si CrossOver lo extrajo; si no, el logo del catálogo o la inicial.
fn app_tile(ui: &mut egui::Ui, a: &Program, brand: Option<(&catalog::App, Option<&PathBuf>)>) -> egui::Response {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(150.0, 128.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 14, if resp.hovered() { p.card_hover } else { p.card });
    let icon = Rect::from_center_size(pos2(rect.center().x, rect.top() + 48.0), vec2(56.0, 56.0));
    match (&a.icon, brand) {
        (Some(path), _) => egui::Image::new(uri(path)).paint_at(ui, icon),
        (None, Some((app, logo))) => discover::brand_icon(ui, icon, app, logo),
        (None, None) => {
            painter.rect_filled(icon, 14, avatar_color(&a.name));
            let initial = a.name.chars().next().unwrap_or('?').to_uppercase().to_string();
            painter.text(icon.center(), Align2::CENTER_CENTER, initial, FontId::new(28.0, demi()), Color32::WHITE);
        }
    }
    painter.text(
        pos2(rect.center().x, rect.bottom() - 20.0),
        Align2::CENTER_CENTER,
        short(&a.name, 18),
        FontId::new(14.0, demi()),
        p.text,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(a.path.display().to_string())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain();
        let mut actions = Vec::new();
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui, &mut actions));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("bottles")
            .resizable(true)
            .default_size(250.0)
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(Margin::symmetric(12, 0)))
            .show(ui, |ui| self.sidebar(ui, &mut actions));
        let frame = egui::Frame::central_panel(ui.style()).inner_margin(Margin::ZERO);
        egui::CentralPanel::default_margins().frame(frame).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if self.creating {
                    self.new_bottle_form(ui, &mut actions);
                } else if self.discover {
                    self.discover_view(ui, &mut actions);
                } else if self.current().is_some() {
                    self.bottle_view(ui, &mut actions);
                } else {
                    self.empty_view(ui, &mut actions);
                }
            });
        });
        self.app_detail(ui.ctx(), &mut actions);
        self.toast_view(ui.ctx(), &mut actions);
        for a in actions {
            self.handle(a);
        }
    }
}

fn report(app: &mut App, r: cx::Result<()>) {
    if let Err(e) = r {
        app.fail(e);
    }
}

fn windows_label(id: &str) -> &str {
    cx::WINDOWS.iter().find(|(w, _)| *w == id).map_or(id, |w| w.1)
}

fn windows_combo(ui: &mut egui::Ui, id: &str, value: &mut String) {
    egui::ComboBox::from_id_salt(id).selected_text(windows_label(value).to_string()).show_ui(ui, |ui| {
        for (w, label) in cx::WINDOWS {
            ui.selectable_value(value, w.to_string(), *label);
        }
    });
}

fn parse_env(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

fn unique(bottles: &[Bottle], base: &str) -> String {
    (1..)
        .map(|i| if i == 1 { base.to_string() } else { format!("{base} {i}") })
        .find(|n| !bottles.iter().any(|b| &b.name == n))
        .unwrap()
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() } else { format!("{}…", s.chars().take(max - 1).collect::<String>()) }
}

/// Diálogos nativos de macOS con `osascript`: sin dependencias extra.
fn osa(script: &str) -> Option<PathBuf> {
    let out = Command::new("osascript").arg("-e").arg(script).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !s.is_empty()).then(|| PathBuf::from(s))
}

fn choose_file(prompt: &str) -> Option<PathBuf> {
    osa(&format!("POSIX path of (choose file with prompt {prompt:?})"))
}

fn choose_save(default_name: &str) -> Option<PathBuf> {
    osa(&format!("POSIX path of (choose file name with prompt \"Exportar bottle\" default name {default_name:?})"))
}
