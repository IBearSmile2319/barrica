//! Catálogo de instalación (el equivalente a los CrossTie de CrossOver, con datos propios).
//!
//! Nada de esto obliga a publicar una versión de Barrica para estar al día:
//! - **El catálogo** (`catalog/catalog.toml`) se descarga al abrir la app; la copia incluida es el respaldo.
//! - **Las versiones** de las apps con `winget` salen de winget-pkgs, que la comunidad y los fabricantes
//!   actualizan a diario (URL y sha256 de cada versión).
//! - **Las apps con `url`** usan el instalador «siempre el último» del fabricante; luego se actualizan solas.
//! - **Los logos** son de Simple Icons (CC0) y se guardan en caché.

use crate::core::{self as cx, Arch, Bottle, Graphics};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Versión del formato que entiende esta Barrica. Un catálogo remoto con otra se ignora.
const FORMAT: u32 = 1;
const BUNDLED: &str = include_str!("../catalog/catalog.toml");
pub const DEFAULT_URL: &str = "https://raw.githubusercontent.com/ibearsmile2319/barrica/master/catalog/catalog.toml";
const ICONS: &str = "https://raw.githubusercontent.com/simple-icons/simple-icons/16.35.0/icons";
const WINGET_API: &str = "https://api.github.com/repos/microsoft/winget-pkgs/contents/manifests";
const WINGET_RAW: &str = "https://raw.githubusercontent.com/microsoft/winget-pkgs/master/manifests";

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct App {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub publisher: String,
    pub category: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    /// Nombre de Simple Icons (`steam`) o una URL a un .svg/.png. Sin icono se dibuja la inicial.
    #[serde(default)]
    pub icon: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub homepage: String,
    /// Instalador fijo del fabricante, que siempre baja la última versión.
    #[serde(default)]
    pub url: Option<String>,
    /// Nombre del archivo, si la URL no lo trae (p. ej. `getInstaller?...`).
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    /// Identificador de winget-pkgs: versión, URL y sha256 se resuelven al instalar.
    #[serde(default)]
    pub winget: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_windows")]
    pub windows: String,
    #[serde(default)]
    pub arch: Arch,
    #[serde(default)]
    pub graphics: Graphics,
    /// Verbos de winetricks (vcrun2022, dotnet48, corefonts…).
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Ejecutables relativos a `drive_c`, para saber si ya está instalada y abrirla. `users/*/` vale por cualquier usuario.
    #[serde(default)]
    pub exe: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

fn default_color() -> String {
    "#B3264F".into()
}
fn default_windows() -> String {
    "win10".into()
}

impl App {
    /// De dónde sale el instalador, para mostrarlo antes de instalar.
    pub fn source(&self) -> String {
        match (&self.winget, &self.url) {
            (Some(id), _) => format!("winget-pkgs · {id}"),
            (None, Some(url)) => host(url).to_string(),
            _ => "?".into(),
        }
    }
    fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty() || [&self.name, &self.publisher, &self.summary, &self.id].iter().any(|s| s.to_lowercase().contains(&q))
    }
}

fn host(url: &str) -> &str {
    let rest = url.split("://").nth(1).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest)
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub updated: String,
    pub categories: Vec<Category>,
    pub apps: Vec<App>,
}

#[derive(Deserialize)]
struct Raw {
    format: u32,
    #[serde(default)]
    updated: String,
    #[serde(default, rename = "category")]
    categories: Vec<Category>,
    #[serde(default, rename = "app")]
    apps: Vec<toml::Value>,
}

impl Catalog {
    /// Cada app se lee por separado: una receta con un campo que esta versión no entiende se salta, no tumba el resto.
    pub fn parse(text: &str) -> cx::Result<Catalog> {
        let raw: Raw = toml::from_str(text).map_err(|e| format!("catálogo inválido: {e}"))?;
        if raw.format != FORMAT {
            return Err(format!("formato de catálogo {} (esta versión entiende el {FORMAT})", raw.format));
        }
        let mut apps: Vec<App> = Vec::new();
        for v in raw.apps {
            if let Ok(app) = v.try_into::<App>()
                && (app.url.is_some() || app.winget.is_some())
                && !apps.iter().any(|a| a.id == app.id)
            {
                apps.push(app);
            }
        }
        Ok(Catalog { updated: raw.updated, categories: raw.categories, apps })
    }

    pub fn app(&self, id: &str) -> Option<&App> {
        self.apps.iter().find(|a| a.id == id)
    }

    pub fn category_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.categories.iter().find(|c| c.id == id).map_or(id, |c| c.name.as_str())
    }

    pub fn search<'a>(&'a self, query: &str, category: Option<&str>) -> Vec<&'a App> {
        self.apps.iter().filter(|a| a.matches(query) && category.is_none_or(|c| a.category == c)).collect()
    }
}

fn dir() -> PathBuf {
    cx::root().join("Catalog")
}

fn bundled() -> Catalog {
    Catalog::parse(BUNDLED).unwrap_or_default()
}

/// El más nuevo entre la copia incluida y la última descargada.
pub fn load() -> Catalog {
    let cached = fs::read_to_string(dir().join("catalog.toml")).ok().and_then(|t| Catalog::parse(&t).ok());
    let b = bundled();
    match cached {
        Some(c) if c.updated >= b.updated => c,
        _ => b,
    }
}

pub fn url() -> String {
    cx::Settings::load().catalog_url.unwrap_or_else(|| DEFAULT_URL.into())
}

/// Descarga el catálogo y lo guarda si es al menos tan nuevo como el incluido.
pub fn refresh() -> cx::Result<Catalog> {
    let text = fetch_text(&url())?;
    let remote = Catalog::parse(&text)?;
    if remote.apps.is_empty() || remote.updated < bundled().updated {
        return Err("el catálogo remoto es más viejo que el incluido".into());
    }
    fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    fs::write(dir().join("catalog.toml"), text).map_err(|e| e.to_string())?;
    Ok(remote)
}

// ---------- Red (con `curl`, que trae macOS) ----------

fn curl() -> Command {
    let mut c = Command::new("curl");
    c.args(["-fL", "--retry", "2", "--connect-timeout", "20", "-A", "Barrica (+https://github.com/ibearsmile2319/barrica)"]);
    c
}

fn fetch_text(url: &str) -> cx::Result<String> {
    let out = curl().args(["-sS", "--max-time", "30", url]).output().map_err(|e| format!("falta curl: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!("no se pudo descargar {url}: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Descarga a `dest` informando de los bytes y, si curl lo sabe, de la fracción.
pub fn download(url: &str, dest: &Path, progress: &dyn Fn(u64, Option<f32>)) -> cx::Result<()> {
    let part = dest.with_extension("part");
    let mut child = curl()
        .args(["--progress-bar", "-o"])
        .arg(&part)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("falta curl: {e}"))?;
    // La barra de curl escribe «###   42.0%» con retornos de carro.
    let mut err = child.stderr.take().expect("stderr");
    let (mut buf, mut line, mut log) = ([0u8; 512], String::new(), String::new());
    while let Ok(n) = err.read(&mut buf) {
        if n == 0 {
            break;
        }
        for ch in String::from_utf8_lossy(&buf[..n]).chars() {
            if ch == '\r' || ch == '\n' {
                if let Some(pct) = line.split_whitespace().last().and_then(|w| w.strip_suffix('%')).and_then(|p| p.parse::<f32>().ok()) {
                    progress(fs::metadata(&part).map(|m| m.len()).unwrap_or(0), Some(pct / 100.0));
                } else if !line.trim().is_empty() {
                    log = line.clone();
                }
                line.clear();
            } else {
                line.push(ch);
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let _ = fs::remove_file(&part);
        // «curl: (22) The requested URL returned error: 404» → «error 404».
        let why = match log.rsplit_once("error: ") {
            Some((_, code)) if code.trim().parse::<u16>().is_ok() => format!("el servidor respondió {}", code.trim()),
            _ if log.contains("(6)") || log.contains("(7)") || log.contains("(28)") => "sin conexión".into(),
            _ => log.trim().trim_start_matches(['#', ' ']).to_string(),
        };
        return Err(format!("No se pudo descargar de {} ({why}). Prueba de nuevo en un rato.", host(url)));
    }
    fs::rename(&part, dest).map_err(|e| e.to_string())
}

pub fn sha256(path: &Path) -> cx::Result<String> {
    let out = Command::new("shasum").args(["-a", "256"]).arg(path).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().next().map(str::to_lowercase).ok_or_else(|| "no se pudo calcular el sha256".into())
}

// ---------- Logos ----------

fn icon_url(app: &App) -> Option<String> {
    match app.icon.as_str() {
        "" => None,
        i if i.contains("://") => Some(i.into()),
        slug => Some(format!("{ICONS}/{slug}.svg")),
    }
}

fn icon_file(app: &App) -> Option<PathBuf> {
    let url = icon_url(app)?;
    let ext = if url.ends_with(".png") { "png" } else { "svg" };
    Some(dir().join("icons").join(format!("{}.{ext}", app.icon.rsplit('/').next()?.split('.').next()?)))
}

/// El logo en caché, si ya está.
pub fn icon(app: &App) -> Option<PathBuf> {
    icon_file(app).filter(|p| p.exists())
}

/// Descarga el logo. Los de Simple Icons son negros: se guardan en blanco para teñirlos al pintarlos.
pub fn fetch_icon(app: &App) -> cx::Result<PathBuf> {
    let (url, path) = icon_url(app).zip(icon_file(app)).ok_or("sin icono")?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    if path.extension().is_some_and(|e| e == "svg") {
        let svg = fetch_text(&url)?;
        if !svg.contains("<svg") {
            return Err("el icono no es un SVG".into());
        }
        fs::write(&path, svg.replacen("<svg ", "<svg fill=\"#FFFFFF\" ", 1)).map_err(|e| e.to_string())?;
    } else {
        download(&url, &path, &|_, _| {})?;
    }
    Ok(path)
}

// ---------- winget-pkgs ----------

/// Un instalador de un manifiesto `*.installer.yaml`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Installer {
    pub arch: String,
    pub kind: String,
    pub url: String,
    pub sha256: String,
    pub locale: String,
}

/// Lo justo de YAML para los manifiestos de winget: claves de primer nivel (valores por defecto)
/// y la lista `Installers:`, donde cada `- ` abre un instalador.
pub fn parse_winget(yaml: &str) -> Vec<Installer> {
    let unquote = |s: &str| s.trim().trim_matches(['\'', '"']).to_string();
    let mut defaults: BTreeMap<String, String> = BTreeMap::new();
    let mut entries: Vec<BTreeMap<String, String>> = Vec::new();
    let mut in_list = false;
    for line in yaml.lines() {
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && !line.starts_with("- ") {
            in_list = line.starts_with("Installers:");
            if let Some((k, v)) = line.split_once(':') {
                defaults.insert(k.trim().into(), unquote(v));
            }
            continue;
        }
        if !in_list {
            continue;
        }
        let body = if let Some(rest) = line.strip_prefix("- ") {
            entries.push(BTreeMap::new());
            rest
        } else if indent == 2 && !line.trim_start().starts_with('-') {
            line.trim_start()
        } else {
            continue; // listas y mapas anidados (códigos de salida, switches…)
        };
        if let (Some(e), Some((k, v))) = (entries.last_mut(), body.split_once(':')) {
            e.insert(k.trim().into(), unquote(v));
        }
    }
    entries
        .into_iter()
        .map(|e| {
            let get = |k: &str| e.get(k).or_else(|| defaults.get(k)).cloned().unwrap_or_default();
            Installer {
                arch: get("Architecture"),
                kind: get("InstallerType").to_lowercase(),
                url: get("InstallerUrl"),
                sha256: get("InstallerSha256").to_lowercase(),
                locale: get("InstallerLocale").to_lowercase(),
            }
        })
        .filter(|i| !i.url.is_empty())
        .collect()
}

/// El mejor instalador para Wine: .exe o .msi (nada de msix ni zip), de la arquitectura del bottle y en español si hay.
pub fn pick(list: &[Installer], arch: Arch) -> Option<Installer> {
    let score = |i: &Installer| -> Option<u32> {
        let a = match (arch, i.arch.as_str()) {
            (Arch::Win64, "x64") => 3,
            (_, "x86" | "neutral" | "") => 2,
            _ => return None,
        };
        let kind = match i.kind.as_str() {
            "exe" | "nullsoft" | "inno" | "burn" => 2,
            "msi" | "wix" => 1,
            _ => return None,
        };
        let locale = match i.locale.as_str() {
            l if l.starts_with("es") => 3,
            "" => 2,
            l if l.starts_with("en") => 2,
            _ => 0,
        };
        Some(a * 100 + locale * 10 + kind)
    };
    list.iter().filter_map(|i| Some((score(i)?, i))).max_by_key(|(s, _)| *s).map(|(_, i)| i.clone())
}

/// Compara versiones por sus números (`2.10.91.91` > `2.9.0`).
pub fn cmp_version(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| -> Vec<u64> { s.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse().ok()).collect() };
    parts(a).cmp(&parts(b)).then_with(|| b.len().cmp(&a.len())) // «1.0» antes que «1.0-beta»
}

fn winget_path(id: &str) -> String {
    let first = id.chars().next().unwrap_or('_').to_ascii_lowercase();
    format!("{first}/{}", id.replace('.', "/"))
}

#[derive(Deserialize)]
struct Entry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

/// La versión más nueva publicada en winget-pkgs y su mejor instalador.
fn winget_latest(id: &str, arch: Arch) -> cx::Result<(String, Installer)> {
    let listing = fetch_text(&format!("{WINGET_API}/{}", winget_path(id))).map_err(|e| {
        if e.contains("403") || e.contains("429") {
            "GitHub limita las consultas sin cuenta (60 por hora). Prueba de nuevo en un rato.".to_string()
        } else {
            e
        }
    })?;
    let entries: Vec<Entry> = serde_json::from_str(&listing).map_err(|_| format!("winget-pkgs no tiene «{id}»"))?;
    // Junto a las versiones hay subpaquetes (`PlugIns`, `RC`): las versiones empiezan por un número.
    let mut versions: Vec<String> = entries
        .into_iter()
        .filter(|e| e.kind == "dir" && e.name.starts_with(|c: char| c.is_ascii_digit()))
        .map(|e| e.name)
        .collect();
    versions.sort_by(|a, b| cmp_version(b, a));
    let version = versions.into_iter().next().ok_or_else(|| format!("winget-pkgs no tiene versiones de «{id}»"))?;
    let base = format!("{WINGET_RAW}/{}/{version}/{id}", winget_path(id));
    let yaml = fetch_text(&format!("{base}.installer.yaml")).or_else(|_| fetch_text(&format!("{base}.yaml")))?;
    let installer = pick(&parse_winget(&yaml), arch)
        .ok_or_else(|| format!("{id} {version} no tiene un instalador .exe o .msi para Wine"))?;
    Ok((version, installer))
}

/// Lo que hay que descargar para instalar.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub url: String,
    pub sha256: Option<String>,
    pub file: String,
    pub version: Option<String>,
}

pub fn resolve(app: &App, arch: Arch) -> cx::Result<Resolved> {
    let (url, sha256, version, kind) = match (&app.winget, &app.url) {
        (Some(id), _) => {
            let (v, i) = winget_latest(id, arch)?;
            (i.url, Some(i.sha256).filter(|s| !s.is_empty()), Some(v), i.kind)
        }
        (None, Some(url)) => (url.clone(), app.sha256.clone(), None, String::new()),
        _ => return Err("la receta no dice de dónde descargar".into()),
    };
    let ext = if matches!(kind.as_str(), "msi" | "wix") { "msi" } else { "exe" };
    let file = app.file.clone().unwrap_or_else(|| file_name(&url, &app.id, ext));
    Ok(Resolved { url, sha256, file, version })
}

/// Nombre del instalador según la URL. SourceForge y compañía acaban en `/download`: vale el segmento anterior.
fn file_name(url: &str, id: &str, ext: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/')
        .take(2)
        .find(|s| {
            let s = s.to_lowercase();
            s.ends_with(".exe") || s.ends_with(".msi")
        })
        .map(decode)
        .filter(|s| !s.contains('/'))
        .unwrap_or_else(|| format!("{id}-setup.{ext}"))
}

/// `%20` y compañía.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---------- Instalar ----------

/// Dónde instalar.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Un bottle nuevo con los ajustes de la receta (lo recomendado: si algo sale mal, no toca nada más).
    New(String),
    Existing(String),
}

/// En qué va la instalación.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Resolving,
    Downloading { bytes: u64, fraction: Option<f32> },
    Verifying,
    Creating,
    Dependencies(String),
    Installing,
}

impl Step {
    pub fn label(&self) -> String {
        match self {
            Step::Resolving => "Buscando la última versión…".into(),
            Step::Downloading { bytes, fraction: Some(f) } => format!("Descargando… {:.0} % · {}", f * 100.0, cx::human(*bytes)),
            Step::Downloading { bytes, .. } => format!("Descargando… {}", cx::human(*bytes)),
            Step::Verifying => "Comprobando la descarga…".into(),
            Step::Creating => "Preparando Windows…".into(),
            Step::Dependencies(d) => format!("Instalando dependencias ({d})…"),
            Step::Installing => "Sigue el instalador…".into(),
        }
    }
    /// Cuánto lleva, si se sabe (solo la descarga): el resto depende de Wine y del instalador.
    pub fn fraction(&self) -> Option<f32> {
        match self {
            Step::Downloading { fraction, .. } => *fraction,
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub bottle: String,
    pub version: Option<String>,
    pub warnings: Vec<String>,
}

pub fn install(app: &App, target: &Target, step: &dyn Fn(Step)) -> cx::Result<Outcome> {
    let mut warnings = Vec::new();
    // Primero la descarga: si falla, no queda un bottle a medias.
    let arch = match target {
        Target::New(_) => app.arch,
        Target::Existing(name) => cx::get(name)?.config.arch,
    };
    step(Step::Resolving);
    let r = resolve(app, arch)?;
    let folder = cx::root().join("Downloads").join(&app.id);
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let setup = folder.join(&r.file);
    download(&r.url, &setup, &|bytes, fraction| step(Step::Downloading { bytes, fraction }))?;
    if let Some(expected) = &r.sha256 {
        step(Step::Verifying);
        let got = sha256(&setup)?;
        if !got.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_dir_all(&folder);
            return Err(format!(
                "La descarga de {} no coincide con el sha256 publicado. No se instaló nada; prueba más tarde.",
                app.name
            ));
        }
    }

    let b = match target {
        Target::Existing(name) => cx::get(name)?,
        Target::New(name) => {
            step(Step::Creating);
            let mut b = cx::create(name, &app.windows, app.arch)?;
            b.config.graphics = app.graphics;
            b.config.env.extend(app.env.clone());
            b.config.description = app.name.clone();
            b.save()?;
            b
        }
    };

    if !app.deps.is_empty() {
        let deps = app.deps.join(", ");
        step(Step::Dependencies(deps.clone()));
        if let Err(e) = cx::winetricks(&b, &app.deps) {
            warnings.push(format!("Sin dependencias ({deps}): {e}"));
        }
    }

    step(Step::Installing);
    let (mut child, log) = cx::launch(&b, &setup, &app.args, None)?;
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&folder);
    if !status.success() && find_in(app, &b).is_none() {
        return Err(format!("El instalador de {} terminó con {status}. Detalles: {}", app.name, log.display()));
    }
    Ok(Outcome { bottle: b.name, version: r.version, warnings })
}

/// El ejecutable de la app en este bottle, si está instalada.
pub fn find_in(app: &App, b: &Bottle) -> Option<PathBuf> {
    let c = b.drive_c();
    app.exe.iter().find_map(|rel| match rel.split_once("users/*/") {
        Some((before, after)) => fs::read_dir(c.join(before).join("users"))
            .ok()?
            .flatten()
            .map(|u| u.path().join(after))
            .find(|p| p.exists()),
        None => Some(c.join(rel)).filter(|p| p.exists()),
    })
}

/// Bottle y ejecutable de cada app ya instalada.
pub fn installed(catalog: &Catalog, bottles: &[Bottle]) -> BTreeMap<String, (String, PathBuf)> {
    catalog
        .apps
        .iter()
        .filter_map(|a| bottles.iter().find_map(|b| Some((a.id.clone(), (b.name.clone(), find_in(a, b)?)))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid() {
        let c = Catalog::parse(BUNDLED).unwrap();
        let raw_count = BUNDLED.matches("[[app]]").count();
        assert_eq!(c.apps.len(), raw_count, "alguna receta no se pudo leer");
        for a in &c.apps {
            assert!(a.url.is_some() != a.winget.is_some(), "{}: url o winget, solo uno", a.id);
            assert!(c.categories.iter().any(|k| k.id == a.category), "{}: categoría «{}»", a.id, a.category);
            assert!(cx::WINDOWS.iter().any(|(w, _)| *w == a.windows), "{}: windows", a.id);
            assert!(a.color.len() == 7 && u32::from_str_radix(&a.color[1..], 16).is_ok(), "{}: color", a.id);
            assert!(!a.exe.is_empty(), "{}: sin exe no se sabe si está instalada", a.id);
        }
        assert!(c.apps.iter().any(|a| a.featured));
        assert_eq!(c.search("valve", None)[0].id, "steam");
        assert!(c.search("", Some("tiendas")).iter().all(|a| a.category == "tiendas"));
    }

    #[test]
    fn lenient_parsing() {
        let text = r#"format = 1
updated = "2030-01-01"
[[app]]
id = "a"
name = "A"
category = "x"
summary = "s"
url = "https://example.com/a.exe"
graphics = "algo-del-futuro"
[[app]]
id = "b"
name = "B"
category = "x"
summary = "s"
winget = "B.B"
campo_nuevo = 3
"#;
        let c = Catalog::parse(text).unwrap();
        assert_eq!(c.apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["b"]);
        assert!(Catalog::parse("format = 2\n").is_err());
    }

    // Recortado de manifiestos reales de winget-pkgs.
    const WINGET: &str = r#"# yaml-language-server: $schema=...
PackageIdentifier: Notepad++.Notepad++
InstallerType: nullsoft
InstallerSwitches:
  Silent: /S
Installers:
- Architecture: x64
  InstallerUrl: https://example.com/npp.x64.exe
  InstallerSha256: 26F3BCCED788
  ExpectedReturnCodes:
  - InstallerReturnCode: 5
    ReturnResponse: packageInUse
- Architecture: x86
  InstallerUrl: https://example.com/npp.exe
  InstallerSha256: EE848AA54DF1
- Architecture: x64
  InstallerType: wix
  InstallerUrl: https://example.com/npp.x64.msi
  InstallerSha256: 0C4F
- Architecture: x64
  InstallerType: zip
  InstallerUrl: https://example.com/npp.zip
  InstallerSha256: 65D3
- InstallerLocale: ar
  Architecture: x64
  InstallerUrl: https://example.com/npp-ar.exe
  InstallerSha256: 1111
ManifestType: installer
"#;

    #[test]
    fn winget_manifests() {
        let list = parse_winget(WINGET);
        assert_eq!(list.len(), 5);
        assert_eq!((list[0].kind.as_str(), list[0].sha256.as_str()), ("nullsoft", "26f3bcced788"));
        assert_eq!(list[2].kind, "wix");
        assert_eq!(pick(&list, Arch::Win64).unwrap().url, "https://example.com/npp.x64.exe");
        assert_eq!(pick(&list, Arch::Win32).unwrap().url, "https://example.com/npp.exe");
        let only_zip = [Installer { arch: "x64".into(), kind: "zip".into(), url: "u".into(), ..Default::default() }];
        assert!(pick(&only_zip, Arch::Win64).is_none());

        let mut v = vec!["2.9.0", "2.10.91.91", "10.0", "2.10.91.91-beta"];
        v.sort_by(|a, b| cmp_version(b, a));
        assert_eq!(v, ["10.0", "2.10.91.91", "2.10.91.91-beta", "2.9.0"]);
        assert_eq!(winget_path("Notepad++.Notepad++"), "n/Notepad++/Notepad++");
        assert_eq!(winget_path("7zip.7zip"), "7/7zip/7zip");

        assert_eq!(file_name("https://sf.net/WinSCP/6.5.7/WinSCP-6.5.7-Setup.exe/download", "w", "exe"), "WinSCP-6.5.7-Setup.exe");
        assert_eq!(file_name("https://x/EpicInstaller.msi?launcherfilename=a.msi", "e", "msi"), "EpicInstaller.msi");
        assert_eq!(file_name("https://itch.io/app/download?platform=windows", "itch", "exe"), "itch-setup.exe");
        assert_eq!(file_name("file:///tmp/Mi%20Setup.exe", "x", "exe"), "Mi Setup.exe");
        assert_eq!(host("https://cdn.akamai.steamstatic.com/client/x.exe"), "cdn.akamai.steamstatic.com");
    }
}
