//! Núcleo de Barrica: bottles de Wine para macOS, sin depender de CrossOver.
//!
//! Un bottle es un WINEPREFIX normal (`drive_c`, `*.reg`, `dosdevices`) más `barrica.toml`.
//! Igual que en `cxbottle.conf` de CrossOver, los ajustes gráficos son variables de entorno,
//! así que sirve cualquier motor Wine de código abierto.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

// ponytail: errores como texto; CLI y GUI solo los muestran.
pub type Result<T> = std::result::Result<T, String>;

fn ctx<E: std::fmt::Display>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e}")
}

// ---------- Rutas ----------

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into())
}

/// Raíz de datos. `BARRICA_HOME` la cambia (tests, o bottles en otro disco).
pub fn root() -> PathBuf {
    std::env::var_os("BARRICA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("Library/Application Support/Barrica"))
}
pub fn bottles_dir() -> PathBuf {
    root().join("Bottles")
}
fn snapshots_dir(name: &str) -> PathBuf {
    root().join("Snapshots").join(name)
}
pub fn logs_dir(name: &str) -> PathBuf {
    root().join("Logs").join(name)
}
pub fn crossover_dir() -> PathBuf {
    std::env::var_os("BARRICA_CROSSOVER")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("Library/Application Support/CrossOver/Bottles"))
}
/// CrossOver.app: la de /Applications o, si no está, la copia de seguridad más nueva en `Backups/`.
/// Es la misma app firmada, así que su `wine` sigue haciendo el control de licencia. Nunca va
/// dentro de Barrica.app: eso sería redistribuirla (licencia de CrossOver, sección 3b).
pub fn crossover_app() -> PathBuf {
    pick_crossover(Path::new("/Applications/CrossOver.app"), &root().join("Backups"))
}
fn pick_crossover(installed: &Path, backups: &Path) -> PathBuf {
    if installed.exists() {
        return installed.into();
    }
    let mut v: Vec<PathBuf> = fs::read_dir(backups)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.join("Contents/SharedSupport/CrossOver/bin/wine").exists()).collect())
        .unwrap_or_default();
    v.sort(); // ponytail: alfabético, 26.10 quedaría antes que 26.3; semver si llega a haber varias
    v.pop().unwrap_or_else(|| installed.into())
}
fn crossover_bin() -> PathBuf {
    std::env::var_os("BARRICA_CROSSOVER_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| crossover_app().join("Contents/SharedSupport/CrossOver/bin"))
}

// ---------- Ajustes globales y motores ----------

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Settings {
    pub engine: Option<PathBuf>,
    pub default_bottle: Option<String>,
}

impl Settings {
    pub fn load() -> Self {
        fs::read_to_string(root().join("settings.toml"))
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(root()).map_err(ctx("crear carpeta de datos"))?;
        let text = toml::to_string(self).map_err(ctx("ajustes"))?;
        fs::write(root().join("settings.toml"), text).map_err(ctx("guardar ajustes"))
    }
}

/// Motores de código abierto que se detectan solos. No incluye el Wine interno de CrossOver
/// a propósito: es de pago y usarlo fuera de su app se saltaría su licencia.
const ENGINE_CANDIDATES: &[(&str, &str)] = &[
    ("/Applications/Wine Stable.app/Contents/Resources/wine/bin/wine", "Wine Stable (WineHQ)"),
    ("/Applications/Wine Devel.app/Contents/Resources/wine/bin/wine", "Wine Devel (WineHQ)"),
    ("/Applications/Wine Staging.app/Contents/Resources/wine/bin/wine", "Wine Staging (WineHQ)"),
    ("/opt/homebrew/bin/wine", "Homebrew"),
    ("/usr/local/bin/wine", "Homebrew (Intel)"),
    ("/usr/local/opt/game-porting-toolkit/bin/wine64", "Game Porting Toolkit"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Engine {
    pub wine: PathBuf,
    pub label: String,
}

/// Motor que viaja dentro de Barrica.app (`Contents/Resources/engine/`, lo mete `bundle.sh`).
fn bundled_engine() -> Option<Engine> {
    let exe = std::env::current_exe().ok()?;
    let wine = exe.parent()?.parent()?.join("Resources/engine/bin/wine");
    wine.exists().then(|| Engine { wine, label: "Barrica (incluido)".into() })
}

/// Motores instalados aparte, uno por carpeta: `<dir>/<nombre>/bin/wine`.
fn installed_engines(dir: &Path) -> Vec<Engine> {
    let mut v: Vec<Engine> = fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("bin/wine").exists())
                .map(|p| Engine { label: format!("Barrica {}", p.file_name().unwrap_or_default().to_string_lossy()), wine: p.join("bin/wine") })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.label.cmp(&a.label)); // ponytail: orden alfabético inverso, no semver
    v
}

/// El incluido primero (es el que se usa si no se elige otro), luego los instalados, luego los del sistema.
pub fn engines() -> Vec<Engine> {
    let mut found: Vec<Engine> = bundled_engine().into_iter().collect();
    found.extend(installed_engines(&root().join("Engines")));
    found.extend(
        ENGINE_CANDIDATES
            .iter()
            .filter(|(p, _)| Path::new(p).exists())
            .map(|(p, label)| Engine { wine: p.into(), label: label.to_string() }),
    );
    if let Some(custom) = Settings::load().engine
        && !found.iter().any(|e| e.wine == custom)
    {
        found.insert(0, Engine { label: format!("Personalizado ({})", custom.display()), wine: custom });
    }
    found
}

/// Motor del bottle; si no tiene, el de ajustes; si no, el primero detectado.
pub fn engine_for(b: Option<&Bottle>) -> Result<PathBuf> {
    if b.is_some_and(|b| b.linked().is_some()) {
        let wine = crossover_bin().join("wine");
        return if wine.exists() { Ok(wine) } else { Err("El bottle está vinculado a CrossOver, pero CrossOver no está instalado.".into()) };
    }
    b.and_then(|b| b.config.engine.clone())
        .or_else(|| Settings::load().engine)
        .or_else(|| engines().into_iter().next().map(|e| e.wine))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            // Homebrew desactivó las casks de WineHQ el 2026-09-01 (no pasan Gatekeeper).
            "No hay ningún motor Wine. Descárgalo de https://wiki.winehq.org/MacOS y elige su ejecutable `wine`.".into()
        })
}

// ---------- Configuración del bottle ----------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    #[default]
    Win64,
    Win32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Graphics {
    #[default]
    Auto,
    D3dmetal,
    Dxmt,
    Dxvk,
    Wined3d,
}

impl Graphics {
    pub const ALL: [Graphics; 5] = [Graphics::Auto, Graphics::D3dmetal, Graphics::Dxmt, Graphics::Dxvk, Graphics::Wined3d];
    pub fn id(self) -> &'static str {
        match self {
            Graphics::Auto => "auto",
            Graphics::D3dmetal => "d3dmetal",
            Graphics::Dxmt => "dxmt",
            Graphics::Dxvk => "dxvk",
            Graphics::Wined3d => "wined3d",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Graphics::Auto => "Automático",
            Graphics::D3dmetal => "D3DMetal (DirectX 11/12, Game Porting Toolkit)",
            Graphics::Dxmt => "DXMT (DirectX 10/11 a Metal, libre)",
            Graphics::Dxvk => "DXVK (requiere DXVK instalado en el bottle)",
            Graphics::Wined3d => "WineD3D (OpenGL)",
        }
    }
    /// Nombre corto, para cabeceras.
    pub fn short(self) -> &'static str {
        match self {
            Graphics::Auto => "Gráficos automáticos",
            Graphics::D3dmetal => "D3DMetal",
            Graphics::Dxmt => "DXMT",
            Graphics::Dxvk => "DXVK",
            Graphics::Wined3d => "WineD3D",
        }
    }
    pub fn parse(s: &str) -> Option<Graphics> {
        Graphics::ALL.into_iter().find(|g| g.id() == s)
    }
}

/// Versiones que acepta `winecfg -v`.
pub const WINDOWS: &[(&str, &str)] = &[
    ("win11", "Windows 11"),
    ("win10", "Windows 10"),
    ("win81", "Windows 8.1"),
    ("win8", "Windows 8"),
    ("win7", "Windows 7"),
    ("vista", "Windows Vista"),
    ("winxp", "Windows XP"),
    ("win2k", "Windows 2000"),
    ("win98", "Windows 98"),
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BottleConfig {
    pub windows: String,
    #[serde(default)]
    pub arch: Arch,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub graphics: Graphics,
    #[serde(default = "yes")]
    pub msync: bool,
    #[serde(default)]
    pub high_resolution: bool,
    #[serde(default)]
    pub engine: Option<PathBuf>,
    /// Bottle de CrossOver vinculado: corre con el `wine` oficial de CrossOver (su motor, D3DMetal y su licencia).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossover: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

fn yes() -> bool {
    true
}

impl Default for BottleConfig {
    fn default() -> Self {
        Self {
            windows: "win10".into(),
            arch: Arch::Win64,
            description: String::new(),
            graphics: Graphics::Auto,
            msync: true,
            high_resolution: false,
            engine: None,
            crossover: None,
            env: BTreeMap::new(),
        }
    }
}

/// Cambia un ajuste por nombre (para la CLI: `barrica set <bottle> <clave> <valor>`).
pub fn set(c: &mut BottleConfig, key: &str, value: &str) -> Result<()> {
    let flag = || match value {
        "1" | "true" | "si" | "sí" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("Usa sí/no para {key}")),
    };
    match key {
        "windows" if WINDOWS.iter().any(|(w, _)| *w == value) => c.windows = value.into(),
        "windows" => return Err(format!("Versión desconocida «{value}»: {}", ids(WINDOWS))),
        "graphics" => c.graphics = Graphics::parse(value).ok_or("Usa auto, d3dmetal, dxmt, dxvk o wined3d")?,
        "msync" => c.msync = flag()?,
        "high_resolution" => c.high_resolution = flag()?,
        "description" => c.description = value.into(),
        "engine" => c.engine = (!value.is_empty()).then(|| value.into()),
        k if k.starts_with("env.") && value.is_empty() => {
            c.env.remove(&k[4..]);
        }
        k if k.starts_with("env.") => {
            c.env.insert(k[4..].into(), value.into());
        }
        _ => return Err(format!("Clave desconocida «{key}»")),
    }
    Ok(())
}

fn ids(list: &[(&str, &str)]) -> String {
    list.iter().map(|(id, _)| *id).collect::<Vec<_>>().join(", ")
}

// ---------- Bottles ----------

const CONFIG: &str = "barrica.toml";

#[derive(Clone, Debug)]
pub struct Bottle {
    pub name: String,
    pub path: PathBuf,
    pub config: BottleConfig,
}

impl Bottle {
    pub fn drive_c(&self) -> PathBuf {
        self.path.join("drive_c")
    }
    /// Bottle de CrossOver al que apunta, si está vinculado.
    pub fn linked(&self) -> Option<&str> {
        self.config.crossover.as_deref()
    }
    /// Carpeta del bottle en Barrica (en uno vinculado solo guarda barrica.toml).
    fn home(&self) -> PathBuf {
        bottles_dir().join(&self.name)
    }
    pub fn save(&self) -> Result<()> {
        let text = toml::to_string(&self.config).map_err(ctx("config"))?;
        fs::write(self.home().join(CONFIG), text).map_err(ctx("guardar barrica.toml"))
    }
}

pub fn list() -> Vec<Bottle> {
    let Ok(dir) = fs::read_dir(bottles_dir()) else { return vec![] };
    let mut v: Vec<Bottle> = dir.flatten().filter_map(|e| get(&e.file_name().to_string_lossy()).ok()).collect();
    v.sort_by_key(|b| b.name.to_lowercase());
    v
}

pub fn get(name: &str) -> Result<Bottle> {
    let mut path = bottles_dir().join(name);
    let text = fs::read_to_string(path.join(CONFIG)).map_err(|_| format!("No existe el bottle «{name}»"))?;
    let mut config: BottleConfig = toml::from_str(&text).map_err(ctx("barrica.toml inválido"))?;
    if let Some(cx) = config.crossover.clone() {
        // Vinculado: los ajustes mandan desde el cxbottle.conf de CrossOver.
        path = crossover_dir().join(&cx);
        let cx_conf = fs::read_to_string(path.join("cxbottle.conf")).unwrap_or_default();
        let description = std::mem::take(&mut config.description);
        config = BottleConfig { crossover: Some(cx), description, ..from_cxbottle(&cx_conf) };
    }
    Ok(Bottle { name: name.into(), path, config })
}

pub fn check_name(name: &str) -> Result<()> {
    let ok = !name.trim().is_empty() && name.len() <= 64 && !name.starts_with('.') && !name.contains(['/', ':']);
    if ok { Ok(()) } else { Err(format!("Nombre no válido «{name}» (sin «/» ni «:», máx. 64)")) }
}

fn free_path(name: &str) -> Result<PathBuf> {
    check_name(name)?;
    let p = bottles_dir().join(name);
    if p.exists() { Err(format!("Ya existe un bottle llamado «{name}»")) } else { Ok(p) }
}

fn unique_name(base: &str) -> String {
    (1..)
        .map(|i| if i == 1 { base.to_string() } else { format!("{base} {i}") })
        .find(|n| !bottles_dir().join(n).exists())
        .unwrap()
}

/// Crea el prefijo con `wineboot --init` y aplica versión de Windows y modo Retina. Tarda.
pub fn create(name: &str, windows: &str, arch: Arch) -> Result<Bottle> {
    let path = free_path(name)?;
    fs::create_dir_all(&path).map_err(ctx("crear bottle"))?;
    let config = BottleConfig { windows: windows.into(), arch, ..Default::default() };
    let b = Bottle { name: name.into(), path, config };
    let result = b.save().and_then(|_| {
        // Sin los diálogos de Mono/Gecko que lanza wineboot en un prefijo nuevo.
        wait(&b, &["wineboot", "--init"], &[("WINEDLLOVERRIDES", "mscoree,mshtml=")], "crear")?;
        apply_settings(&b)
    });
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&b.path); // es nuestro y está a medias
        return Err(e);
    }
    Ok(b)
}

/// Lo que vive en el registro del prefijo: versión de Windows y modo Retina.
pub fn apply_settings(b: &Bottle) -> Result<()> {
    if b.linked().is_some() {
        return Err("Los ajustes de un bottle vinculado se cambian en CrossOver.".into());
    }
    wait(b, &["winecfg", "-v", &b.config.windows], &[], "winecfg")?;
    let retina = if b.config.high_resolution { "y" } else { "n" };
    let key = r"HKCU\Software\Wine\Mac Driver";
    wait(b, &["reg", "add", key, "/v", "RetinaMode", "/d", retina, "/f"], &[], "registro")?;
    Ok(())
}

pub fn rename(b: &Bottle, to: &str) -> Result<Bottle> {
    let dst = free_path(to)?;
    fs::rename(b.home(), &dst).map_err(ctx("renombrar"))?; // en un vinculado, CrossOver no se entera
    for dir in [snapshots_dir(&b.name), logs_dir(&b.name)] {
        if dir.exists() {
            let _ = fs::rename(&dir, dir.with_file_name(to));
        }
    }
    let mut s = Settings::load();
    if s.default_bottle.as_deref() == Some(&b.name) {
        s.default_bottle = Some(to.into());
        s.save()?;
    }
    get(to)
}

/// Copia con clonefile de APFS (`cp -c`): instantánea y sin gastar disco hasta que algo cambie.
fn clone_dir(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(ctx("crear carpeta"))?;
    }
    let copy = |flags: &str| {
        let ok = Command::new("cp").arg(flags).arg(src).arg(dst).status().is_ok_and(|s| s.success());
        if !ok {
            let _ = fs::remove_dir_all(dst); // si no, el siguiente `cp` copiaría *dentro* de dst
        }
        ok
    };
    if copy("-cR") || copy("-R") { Ok(()) } else { Err(format!("No se pudo copiar {}", src.display())) }
}

pub fn duplicate(b: &Bottle, to: &str) -> Result<Bottle> {
    if b.linked().is_some() {
        return Err("Para copiarlo, duplica el bottle en CrossOver; aquí puedes hacerle un snapshot.".into());
    }
    clone_dir(&b.path, &free_path(to)?)?;
    get(to)
}

/// A la Papelera, nunca borrado: se recupera desde Finder.
fn move_to_trash(p: &Path) -> Result<()> {
    if let Some(dir) = std::env::var_os("BARRICA_TRASH") {
        let name = format!("{} {}", p.file_name().unwrap_or_default().to_string_lossy(), stamp());
        return fs::rename(p, Path::new(&dir).join(name)).map_err(ctx("mover a la Papelera"));
    }
    run_ok(Command::new("/usr/bin/trash").arg(p), "mover a la Papelera")
}

/// En un vinculado solo quita el vínculo: el bottle de CrossOver no se toca.
pub fn trash(b: &Bottle) -> Result<()> {
    move_to_trash(&b.home())
}

// ---------- Snapshots (CrossOver no los tiene) ----------

pub fn snapshot(b: &Bottle, label: &str) -> Result<()> {
    check_name(label)?;
    let dst = snapshots_dir(&b.name).join(label);
    if dst.exists() {
        return Err(format!("Ya hay un snapshot «{label}»"));
    }
    clone_dir(&b.path, &dst)
}

pub fn snapshots(b: &Bottle) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(snapshots_dir(&b.name))
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Vuelve a un snapshot. El estado actual se va a la Papelera, por si acaso.
pub fn restore(b: &Bottle, label: &str) -> Result<()> {
    let src = snapshots_dir(&b.name).join(label);
    if !src.exists() {
        return Err(format!("No hay snapshot «{label}»"));
    }
    let _ = kill(b, false);
    move_to_trash(&b.path)?;
    clone_dir(&src, &b.path)
}

pub fn delete_snapshot(b: &Bottle, label: &str) -> Result<()> {
    move_to_trash(&snapshots_dir(&b.name).join(label))
}

// ---------- Exportar / importar ----------

fn run_ok(cmd: &mut Command, what: &str) -> Result<()> {
    let status = cmd.status().map_err(ctx(what))?;
    if status.success() { Ok(()) } else { Err(format!("{what}: {status}")) }
}

/// Exporta a .tar.gz (el «Exportar bottle a archivo» de CrossOver).
pub fn export(b: &Bottle, dest: &Path) -> Result<()> {
    let (parent, dir) = (b.path.parent().unwrap_or(Path::new("/")), b.path.file_name().unwrap_or_default());
    run_ok(Command::new("tar").arg("-czf").arg(dest).arg("-C").arg(parent).arg(dir), "exportar")
}

/// Importa un archivo de Barrica o un bottle de CrossOver comprimido (tar detecta el formato).
pub fn import_archive(archive: &Path) -> Result<Bottle> {
    let tmp = bottles_dir().join(format!(".import-{}", std::process::id()));
    fs::create_dir_all(&tmp).map_err(ctx("importar"))?;
    let result = (|| {
        run_ok(Command::new("tar").arg("-xf").arg(archive).arg("-C").arg(&tmp), "descomprimir")?;
        let inner = fs::read_dir(&tmp)
            .map_err(ctx("importar"))?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.join("drive_c").exists() || p.join("system.reg").exists())
            .ok_or("El archivo no contiene un bottle")?;
        let name = unique_name(&inner.file_name().unwrap_or_default().to_string_lossy());
        let dst = bottles_dir().join(&name);
        fs::rename(&inner, &dst).map_err(ctx("importar"))?;
        if !dst.join(CONFIG).exists() {
            write_crossover_config(&dst)?;
        }
        get(&name)
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

// ---------- CrossOver ----------

/// Bottles de CrossOver en este Mac.
pub fn crossover_bottles() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(crossover_dir())
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.join("cxbottle.conf").exists()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Vincula un bottle de CrossOver sin copiarlo: Barrica lo abre con el `wine` oficial de CrossOver
/// (su licencia, su motor y D3DMetal) y le suma snapshots, logs por ejecución y su interfaz.
pub fn link_crossover(cx_name: &str, name: &str) -> Result<Bottle> {
    if !crossover_dir().join(cx_name).join("cxbottle.conf").exists() {
        return Err(format!("CrossOver no tiene un bottle «{cx_name}»"));
    }
    let home = free_path(name)?;
    fs::create_dir_all(&home).map_err(ctx("vincular"))?;
    let config = BottleConfig { crossover: Some(cx_name.into()), ..Default::default() };
    Bottle { name: name.into(), path: home, config }.save()?;
    get(name)
}

/// Importa un bottle de CrossOver como clon APFS: el original no se toca.
/// Ojo: lo abrirá otra versión de Wine, que actualizará el prefijo la primera vez.
pub fn import_crossover(src: &Path, name: &str) -> Result<Bottle> {
    let dst = free_path(name)?;
    clone_dir(src, &dst)?;
    write_crossover_config(&dst).inspect_err(|_| {
        let _ = fs::remove_dir_all(&dst);
    })?;
    get(name)
}

fn write_crossover_config(dir: &Path) -> Result<()> {
    let text = fs::read_to_string(dir.join("cxbottle.conf")).unwrap_or_default();
    let name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
    Bottle { name, path: dir.into(), config: from_cxbottle(&text) }.save()
}

/// INI de CrossOver: `[Sección]` y líneas `"Clave" = "Valor"`.
fn parse_cx(text: &str) -> BTreeMap<(String, String), String> {
    let unquote = |s: &str| s.trim().trim_matches('"').to_string();
    let mut section = String::new();
    let mut map = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|l| !l.starts_with([';', '#'])) {
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            map.insert((section.clone(), unquote(k)), unquote(v));
        }
    }
    map
}

/// `cxbottle.conf` → `barrica.toml`: plantilla, arquitectura y variables de entorno.
pub fn from_cxbottle(text: &str) -> BottleConfig {
    let ini = parse_cx(text);
    let get = |s: &str, k: &str| ini.get(&(s.to_string(), k.to_string())).cloned().unwrap_or_default();
    let template = get("Bottle", "Template"); // p. ej. "win10_64"
    let windows = match template.trim_end_matches("_64") {
        "" => "win10",
        "winvista" => "vista",
        "win2000" => "win2k",
        w => w,
    };
    let arch =
        if template.ends_with("_64") || get("Bottle", "WineArch") == "win64" { Arch::Win64 } else { Arch::Win32 };
    let mut c = BottleConfig {
        windows: windows.into(),
        arch,
        msync: false,
        description: get("Bottle", "Description"),
        ..Default::default()
    };
    for ((section, key), value) in &ini {
        if section != "EnvironmentVariables" {
            continue;
        }
        match (key.as_str(), value.as_str()) {
            ("WINEMSYNC", v) => c.msync = v == "1",
            ("WINED3DMETAL", "1") => c.graphics = Graphics::D3dmetal,
            ("CX_GRAPHICS_BACKEND", v) => {
                if let Some(g) = Graphics::parse(&v.to_lowercase()) {
                    c.graphics = g;
                }
            }
            (k, _) if k.starts_with("CX_") || k == "WINED3DMETAL" => {}
            (k, v) => {
                c.env.insert(k.into(), v.into());
            }
        }
    }
    c
}

// ---------- Ejecutar ----------

/// `WINEDLLPATH` del bottle con un motor de Barrica: la capa gráfica elegida y detrás WineD3D.
/// Wine busca cada DLL builtin primero en su carpeta y luego en `WINEDLLPATH`, y engine/build.sh
/// saca de su carpeta las que chocan (d3d10, d3d11, dxgi…). Así cada bottle elige capa sin copiar
/// nada, y lo que la capa no traiga (el d3d10 de DXMT) sale de WineD3D. Las capas viven en el motor
/// (`lib/wine-layers/<capa>`) o en `user_layers/<capa>/wine` (D3DMetal, que es de Apple y conserva
/// la estructura de su redist). Otros motores: None.
fn graphics_dllpath(wine: &Path, g: Graphics, user_layers: &Path) -> Option<String> {
    let layers = wine.parent()?.parent()?.join("lib/wine-layers");
    let wined3d = layers.join("wined3d");
    if !wined3d.is_dir() {
        return None;
    }
    let wanted: &[&str] = match g {
        Graphics::Auto => &["d3dmetal", "dxmt"], // ponytail: D3DMetal primero; perfiles por juego en la fase 2
        Graphics::D3dmetal => &["d3dmetal"],
        Graphics::Dxmt => &["dxmt"],
        Graphics::Dxvk | Graphics::Wined3d => &[],
    };
    let chosen = wanted.iter().flat_map(|l| [layers.join(l), user_layers.join(l).join("wine")]).find(|d| d.is_dir());
    Some(chosen.into_iter().chain([wined3d]).map(|d| d.display().to_string()).collect::<Vec<_>>().join(":"))
}

/// Instala D3DMetal del Game Porting Toolkit de Apple como capa en `Layers/d3dmetal/`, sin tocar sus
/// binarios (su licencia prohíbe modificarlos). `src` es el volumen «Evaluation environment for Windows
/// games», su `redist` o su `redist/lib`. Se copia tal cual: sus `wine/x86_64-unix/*.so` son enlaces a
/// `../../external/libd3dshared.dylib`, así que la estructura `wine/` + `external/` tiene que quedarse.
pub fn install_gptk(src: &Path) -> Result<PathBuf> {
    let lib = [src.to_path_buf(), src.join("lib"), src.join("redist/lib")]
        .into_iter()
        .find(|l| l.join("wine/x86_64-windows/d3d11.dll").exists() && l.join("external/libd3dshared.dylib").exists())
        .ok_or("No encuentro redist/lib del Game Porting Toolkit. Monta «Evaluation environment for Windows games» y pasa esa carpeta.")?;
    let dst = root().join("Layers/d3dmetal");
    let tmp = dst.with_extension("nuevo");
    let _ = fs::remove_dir_all(&tmp);
    run_ok(Command::new("ditto").arg(&lib).arg(&tmp), "copiar el Game Porting Toolkit")?;
    // MetalFX (DLSS → MetalFX), como indica el Léeme de Apple; se activa con D3DM_ENABLE_METALFX=1.
    for (dir, ext) in [("wine/x86_64-windows", "dll"), ("wine/x86_64-unix", "so")] {
        let d = tmp.join(dir);
        let _ = fs::rename(d.join(format!("nvngx-on-metalfx.{ext}")), d.join(format!("nvngx.{ext}")));
    }
    // Avisos de copyright y licencia: la licencia exige conservarlos en cada copia.
    for doc in ["License.rtf", "Acknowledgements.rtf", "Read Me.rtf"] {
        if let Some(f) = lib.ancestors().map(|a| a.join(doc)).find(|f| f.exists()) {
            fs::copy(f, tmp.join(doc)).map_err(ctx("copiar la licencia del GPTK"))?;
        }
    }
    let _ = fs::remove_dir_all(&dst); // una instalación anterior del GPTK, no datos del usuario
    fs::rename(&tmp, &dst).map_err(ctx("instalar D3DMetal"))?;
    Ok(dst)
}

/// Variables de entorno del bottle. Como en CrossOver, los ajustes gráficos son env vars.
pub fn env(b: &Bottle, debug: Option<&str>) -> Vec<(String, String)> {
    let c = &b.config;
    let mut v: Vec<(String, String)> = Vec::new();
    let mut push = |k: &str, val: &str| v.push((k.into(), val.into()));
    push("WINEPREFIX", &b.path.display().to_string());
    push("WINEDEBUG", debug.unwrap_or("-all")); // sin trazas va más rápido
    if c.arch == Arch::Win32 {
        push("WINEARCH", "win32");
    }
    if c.msync {
        push("WINEMSYNC", "1");
        push("WINEESYNC", "1");
    }
    match c.graphics {
        Graphics::Auto | Graphics::Dxmt => {}
        Graphics::D3dmetal => push("WINED3DMETAL", "1"),
        Graphics::Dxvk => push("WINEDLLOVERRIDES", "dxgi,d3d9,d3d10core,d3d11=n,b"),
        Graphics::Wined3d => push("WINEDLLOVERRIDES", "dxgi,d3d9,d3d10core,d3d11=b"),
    }
    if let Some(path) = engine_for(Some(b)).ok().and_then(|wine| graphics_dllpath(&wine, c.graphics, &root().join("Layers"))) {
        push("WINEDLLPATH", &path);
    }
    for (k, val) in &c.env {
        push(k, val); // las del usuario mandan
    }
    v
}

/// `wine <args>` con el entorno del bottle.
pub fn command<S: AsRef<OsStr>>(b: &Bottle, args: &[S], debug: Option<&str>) -> Result<Command> {
    let mut cmd = Command::new(engine_for(Some(b))?);
    match b.linked() {
        // El wrapper de CrossOver arma el entorno desde su cxbottle.conf: no le metemos el nuestro.
        Some(cx) => cmd.arg("--bottle").arg(cx).args(args).envs(debug.map(|d| ("WINEDEBUG", d))),
        None => cmd.args(args).envs(env(b, debug)),
    };
    cmd.current_dir(&b.path);
    Ok(cmd)
}

/// Entorno para una shell. En un vinculado, `CX_BOTTLE` hace que el `wine` de CrossOver lo use.
fn shell_env(b: &Bottle) -> Vec<(String, String)> {
    match b.linked() {
        Some(cx) => vec![("CX_BOTTLE".into(), cx.into())],
        None => env(b, None),
    }
}

pub fn stamp() -> String {
    Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn new_log(b: &Bottle, label: &str) -> Result<PathBuf> {
    let dir = logs_dir(&b.name);
    fs::create_dir_all(&dir).map_err(ctx("carpeta de logs"))?;
    Ok(dir.join(format!("{}-{}.log", stamp(), label.replace(['/', ' '], "_"))))
}

/// Logs del bottle, el más nuevo primero.
pub fn logs(b: &Bottle) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(logs_dir(&b.name))
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "log")).collect())
        .unwrap_or_default();
    v.sort();
    v.reverse();
    v
}

/// Corre `wine <args>` y espera; toda la salida queda en un log.
fn wait(b: &Bottle, args: &[&str], extra_env: &[(&str, &str)], label: &str) -> Result<String> {
    let mut cmd = command(b, args, None)?; // antes del log: sin motor no queda un log vacío
    let log = new_log(b, label)?;
    let out = cmd.envs(extra_env.iter().copied()).output().map_err(ctx("no se pudo ejecutar wine"))?;
    let all = [&out.stdout[..], &out.stderr[..]].concat(); // el wrapper de CrossOver escribe por stderr
    let _ = fs::write(&log, &all);
    if out.status.success() {
        Ok(String::from_utf8_lossy(&all).into_owned())
    } else {
        Err(format!("wine {} falló ({}). Detalles: {}", args.join(" "), out.status, log.display()))
    }
}

/// Lanza `wine <args>` sin esperar, con stdout/stderr en un log propio de esta ejecución.
fn spawn(b: &Bottle, args: &[OsString], label: &str, debug: Option<&str>) -> Result<(Child, PathBuf)> {
    let mut cmd = command(b, args, debug)?;
    let log = new_log(b, label)?;
    let file = fs::File::create(&log).map_err(ctx("crear log"))?;
    let child = cmd
        .stdin(Stdio::null())
        .stdout(file.try_clone().map_err(ctx("log"))?)
        .stderr(file)
        .spawn()
        .map_err(ctx("no se pudo lanzar wine"))?;
    Ok((child, log))
}

/// Nombre del programa para su log: sin carpeta (de macOS o de Windows, `C:\…`) ni extensión.
fn log_label(program: &str) -> String {
    let name = program.rsplit(['/', '\\']).next().unwrap_or(program);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
}

/// Abre un .exe/.msi/.lnk/.bat con `start /wait /unix` (Wine pone la carpeta de trabajo).
/// Cada ejecución deja su log; en CrossOver hay que activar el registro a mano.
pub fn launch(b: &Bottle, program: &Path, args: &[String], debug: Option<&str>) -> Result<(Child, PathBuf)> {
    let label = log_label(&program.to_string_lossy());
    let mut full: Vec<OsString> = ["start", "/wait", "/unix"].map(OsString::from).into();
    full.push(program.as_os_str().into());
    full.extend(args.iter().map(OsString::from));
    spawn(b, &full, &label, debug)
}

/// Herramientas de Wine: winecfg, regedit, taskmgr, control, explorer, uninstaller.
pub fn tool(b: &Bottle, name: &str) -> Result<(Child, PathBuf)> {
    spawn(b, &[OsString::from(name)], name, None)
}

/// Cierra los procesos del bottle (`wineserver -k`) o los mata (`-k9`).
pub fn kill(b: &Bottle, force: bool) -> Result<()> {
    let server = engine_for(Some(b))?.with_file_name("wineserver");
    // Sale con error si no había nada abierto: eso no es un fallo.
    let _ = Command::new(server).arg(if force { "-k9" } else { "-k" }).envs(env(b, None)).status();
    Ok(())
}

/// Software instalado según el registro (`uninstaller --list`): (id, nombre).
pub fn installed(b: &Bottle) -> Result<Vec<(String, String)>> {
    Ok(parse_uninstaller(&wait(b, &["uninstaller", "--list"], &[], "uninstaller")?))
}

fn parse_uninstaller(out: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> =
        out.lines().filter_map(|l| l.split_once("|||")).map(|(id, n)| (id.trim().into(), n.trim().into())).collect();
    v.sort_by_key(|(_, name)| name.to_lowercase());
    v.dedup();
    v
}

pub fn uninstall(b: &Bottle, id: &str) -> Result<(Child, PathBuf)> {
    spawn(b, &["uninstaller", "--remove", id].map(OsString::from), "desinstalar", None)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub name: String,
    pub path: PathBuf,
    pub args: Vec<String>,
    /// Icono cuadrado (los que CrossOver extrae de los .exe).
    pub icon: Option<PathBuf>,
    /// Portada vertical y banner de los juegos de Steam (su caché de biblioteca).
    pub cover: Option<PathBuf>,
    pub hero: Option<PathBuf>,
}

/// Accesos directos del menú Inicio y del Escritorio: los launchers de CrossOver (cxmenu.conf).
pub fn programs(b: &Bottle) -> Vec<Program> {
    let c = b.drive_c();
    let mut roots = vec![c.join("ProgramData/Microsoft/Windows/Start Menu/Programs")];
    if let Ok(users) = fs::read_dir(c.join("users")) {
        for u in users.flatten() {
            roots.push(u.path().join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs"));
            roots.push(u.path().join("Desktop"));
        }
    }
    let icons = lnk_icons(b);
    let mut found = Vec::new();
    for r in roots {
        collect_lnk(&r, &mut found, &icons, 0);
    }
    // Juegos de Steam: un launcher directo por juego (`steam.exe -applaunch <id>`), que CrossOver no da.
    let steam = c.join("Program Files (x86)/Steam");
    for e in fs::read_dir(steam.join("steamapps")).into_iter().flatten().flatten() {
        if !e.file_name().to_string_lossy().starts_with("appmanifest_") {
            continue;
        }
        let text = fs::read_to_string(e.path()).unwrap_or_default();
        if let (Some(id), Some(name)) = (acf(&text, "appid"), acf(&text, "name"))
            && !name.starts_with("Steamworks")
        {
            let art = steam.join("appcache/librarycache").join(&id);
            found.push(Program {
                name,
                path: steam.join("steam.exe"),
                cover: find_art(&art, "library_600x900.jpg"),
                hero: find_art(&art, "library_hero.jpg"),
                icon: None,
                args: vec!["-applaunch".into(), id],
            });
        }
    }
    found.sort_by_key(|p| p.name.to_lowercase());
    found.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    found
}

/// Valor de una clave en un .acf de Steam: líneas `"clave"  "valor"`.
fn acf(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let mut parts = l.split('"').filter(|p| !p.trim().is_empty());
        (parts.next()? == key).then(|| parts.next().map(String::from)).flatten()
    })
}

/// Un archivo de arte de Steam: en la carpeta del juego o en una subcarpeta (Steam nuevo usa hashes).
fn find_art(dir: &Path, file: &str) -> Option<PathBuf> {
    let direct = dir.join(file);
    if direct.exists() {
        return Some(direct);
    }
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path().join(file)).find(|p| p.exists())
}

/// Iconos que CrossOver ya extrajo: `cxmenu.conf` dice qué icono usa cada .lnk.
fn lnk_icons(b: &Bottle) -> BTreeMap<String, PathBuf> {
    let conf = fs::read_to_string(b.path.join("cxmenu.conf")).unwrap_or_default();
    let base = b.path.join("windata/cxmenu/icons/hicolor");
    parse_cx(&conf)
        .into_iter()
        .filter(|((_, key), _)| key == "Icon")
        .filter_map(|((section, _), icon)| {
            let lnk = section.rsplit('/').next()?.to_lowercase();
            let png = ["256x256", "128x128", "64x64", "48x48", "40x40", "32x32"]
                .iter()
                .map(|size| base.join(size).join("apps").join(format!("{icon}.png")))
                .find(|p| p.exists())?;
            Some((lnk, png))
        })
        .collect()
}

/// ¿Hay un wineserver vivo para este bottle? Wine crea `/tmp/.wine-<uid>/server-<dev>-<inode>/socket`.
pub fn is_running(b: &Bottle) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(m) = fs::metadata(&b.path) else { return false };
    Path::new(&format!("/tmp/.wine-{}/server-{:x}-{:x}/socket", m.uid(), m.dev(), m.ino())).exists()
}

fn collect_lnk(dir: &Path, out: &mut Vec<Program>, icons: &BTreeMap<String, PathBuf>, depth: u8) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 4 {
            collect_lnk(&p, out, icons, depth + 1);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
            let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if !lower.starts_with("uninstall") && !lower.starts_with("desinstalar") {
                let file = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                let icon = icons.get(&file).cloned();
                out.push(Program { name, path: p, args: vec![], icon, cover: None, hero: None });
            }
        }
    }
}

/// Abre una terminal con el entorno del bottle. Usa un .command, así no pide permisos de AppleScript.
pub fn terminal(b: &Bottle) -> Result<()> {
    let wine = engine_for(Some(b))?;
    let q = |s: &str| format!("'{}'", s.replace('\'', r"'\''"));
    let mut script = String::from("#!/bin/sh\n");
    for (k, v) in shell_env(b) {
        script += &format!("export {k}={}\n", q(&v));
    }
    script += &format!(
        "export PATH={}:\"$PATH\"\ncd {}\necho {}\nexec \"${{SHELL:-/bin/zsh}}\"\n",
        q(&wine.parent().unwrap_or(Path::new("/")).display().to_string()),
        q(&b.drive_c().display().to_string()),
        q(&format!("Bottle «{}»: wine, winecfg y regedit ya apuntan aquí.", b.name)),
    );
    let file = logs_dir(&b.name).join("terminal.command");
    fs::create_dir_all(logs_dir(&b.name)).map_err(ctx("terminal"))?;
    fs::write(&file, script).map_err(ctx("terminal"))?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).map_err(ctx("terminal"))?;
    open(&file)
}

/// Shell en la terminal actual (CLI).
pub fn shell(b: &Bottle) -> Result<()> {
    let wine = engine_for(Some(b))?;
    let path = format!("{}:{}", wine.parent().unwrap_or(Path::new("/")).display(), std::env::var("PATH").unwrap_or_default());
    let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    run_ok(Command::new(sh).envs(shell_env(b)).env("PATH", path).current_dir(b.drive_c()), "shell")
}

pub fn winetricks(b: &Bottle, verbs: &[String]) -> Result<()> {
    if b.linked().is_some() {
        return Err("En un bottle vinculado, instala las dependencias desde CrossOver.".into());
    }
    let wine = engine_for(Some(b))?;
    let status = Command::new("winetricks")
        .arg("-q")
        .args(verbs)
        .envs(env(b, None))
        .env("WINESERVER", wine.with_file_name("wineserver"))
        .env("WINE", &wine)
        .status()
        .map_err(|_| "Falta winetricks: brew install winetricks".to_string())?;
    if status.success() { Ok(()) } else { Err(format!("winetricks terminó con {status}")) }
}

pub fn open(p: &Path) -> Result<()> {
    run_ok(Command::new("open").arg(p), "abrir")
}

/// Tamaño en disco (`du -sk`).
pub fn size(path: &Path) -> Option<u64> {
    let out = Command::new("du").arg("-sk").arg(path).output().ok()?;
    let kb: u64 = String::from_utf8_lossy(&out.stdout).split_whitespace().next()?.parse().ok()?;
    Some(kb * 1024)
}

pub fn human(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 1.0 { format!("{gb:.1} GB") } else { format!("{:.0} MB", bytes as f64 / 1e6) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortado de un cxbottle.conf real de CrossOver 26.3.
    const CX: &str = r#"[Bottle]
"BottleID" = "0D7BC615-CC91-422F-9438-51B4A5B0FF7F"
"Template" = "win10_64"
"WineArch" = "win64"
[ManagedUpdatePolicy]
"^cxbottle.conf$"     = "Link,ReplaceFiles"
[EnvironmentVariables]
;;"PROMPT" = "$p$g"
"CX_BOTTLE_CREATOR_APPID" = "com.codeweavers.c4.10470"
"WINEMSYNC" = "1"
"WINED3DMETAL" = "1"
"CX_GRAPHICS_BACKEND" = ""
"WINEDLLOVERRIDES" = "winemenubuilder.exe=d"
"#;

    #[test]
    fn installed_engines_are_listed() {
        let tmp = std::env::temp_dir().join(format!("barrica-engines-{}", std::process::id()));
        let wine = tmp.join("cx-26.3/bin/wine");
        fs::create_dir_all(wine.parent().unwrap()).unwrap();
        fs::write(&wine, "").unwrap();
        fs::create_dir_all(tmp.join("sources")).unwrap(); // sin bin/wine: no es un motor
        let v = installed_engines(&tmp);
        assert_eq!(v, vec![Engine { wine: wine.clone(), label: "Barrica cx-26.3".into() }]);
        assert!(installed_engines(&tmp.join("no-existe")).is_empty());

        // Capas: la elegida y detrás WineD3D; un motor sin capas (WineHQ) no toca WINEDLLPATH.
        let user = tmp.join("Layers");
        assert_eq!(graphics_dllpath(&wine, Graphics::Auto, &user), None);
        let layers = tmp.join("cx-26.3/lib/wine-layers");
        for l in ["wined3d", "dxmt"] {
            fs::create_dir_all(layers.join(l)).unwrap();
        }
        let path = |g| {
            let p = graphics_dllpath(&wine, g, &user).unwrap();
            let name = |d: &str| d.trim_end_matches("/wine").rsplit('/').next().unwrap().to_string();
            p.split(':').map(name).collect::<Vec<_>>().join(":")
        };
        assert_eq!(path(Graphics::Auto), "dxmt:wined3d"); // sin D3DMetal instalado
        assert_eq!(path(Graphics::D3dmetal), "wined3d");
        assert_eq!(path(Graphics::Wined3d), "wined3d");
        fs::create_dir_all(user.join("d3dmetal/wine")).unwrap(); // instalado por el usuario
        assert_eq!(path(Graphics::Auto), "d3dmetal:wined3d");
        assert_eq!(path(Graphics::Dxmt), "dxmt:wined3d");

        // CrossOver: la instalada manda; si falta, el respaldo.
        let backup = tmp.join("Backups/CrossOver-26.3.0.app");
        let cx_wine = backup.join("Contents/SharedSupport/CrossOver/bin/wine");
        fs::create_dir_all(cx_wine.parent().unwrap()).unwrap();
        fs::write(&cx_wine, "").unwrap();
        assert_eq!(pick_crossover(&tmp.join("no-existe.app"), &tmp.join("Backups")), backup);
        assert_eq!(pick_crossover(&tmp, &tmp.join("Backups")), tmp); // tmp existe: «instalada»
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn crossover_config() {
        let c = from_cxbottle(CX);
        assert_eq!((c.windows.as_str(), c.arch, c.graphics, c.msync), ("win10", Arch::Win64, Graphics::D3dmetal, true));
        assert_eq!(c.env.len(), 1);
        assert_eq!(c.env["WINEDLLOVERRIDES"], "winemenubuilder.exe=d"); // el «=» del valor sobrevive
        let xp = from_cxbottle("[Bottle]\n\"Template\" = \"winxp\"\n");
        assert_eq!((xp.windows.as_str(), xp.arch, xp.msync), ("winxp", Arch::Win32, false));
        assert_eq!(from_cxbottle("[Bottle]\n\"Template\" = \"winvista_64\"\n").windows, "vista");
    }

    #[test]
    fn env_and_settings() {
        assert_eq!(log_label(r"C:\windows\notepad.exe"), "notepad");
        assert_eq!(log_label("/Users/a/Setup v2.1.msi"), "Setup v2.1");
        let mut b = Bottle { name: "x".into(), path: "/b/x".into(), config: BottleConfig::default() };
        set(&mut b.config, "graphics", "dxvk").unwrap();
        set(&mut b.config, "env.DXVK_HUD", "fps").unwrap();
        set(&mut b.config, "msync", "no").unwrap();
        assert!(set(&mut b.config, "windows", "win95").is_err());
        let e = env(&b, None);
        assert!(e.contains(&("WINEPREFIX".into(), "/b/x".into())));
        assert!(e.contains(&("WINEDLLOVERRIDES".into(), "dxgi,d3d9,d3d10core,d3d11=n,b".into())));
        assert!(e.contains(&("DXVK_HUD".into(), "fps".into())));
        assert!(!e.iter().any(|(k, _)| k == "WINEMSYNC"));
        assert!(check_name("a/b").is_err() && check_name(".x").is_err() && check_name("Juegos").is_ok());
        assert_eq!(parse_uninstaller("{A}|||Steam\nbasura\n"), vec![("{A}".into(), "Steam".into())]);
    }

    /// Ciclo completo con un `wine` falso que anota cómo lo llaman.
    #[test]
    fn lifecycle() {
        let tmp = std::env::temp_dir().join(format!("barrica-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let bin = tmp.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(tmp.join("trash")).unwrap();
        let fake = "#!/bin/sh\necho \"$*\" >> \"$BARRICA_HOME/calls.txt\"\n\
                    [ \"$1\" = uninstaller ] && echo '{ABC}|||Test App'\nexit 0\n";
        for name in ["wine", "wineserver"] {
            fs::write(bin.join(name), fake).unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        // SAFETY: es el único test que toca el entorno.
        unsafe {
            std::env::set_var("BARRICA_HOME", tmp.join("data"));
            std::env::set_var("BARRICA_TRASH", tmp.join("trash"));
        }
        Settings { engine: Some(bin.join("wine")), default_bottle: None }.save().unwrap();
        let calls = || fs::read_to_string(tmp.join("data/calls.txt")).unwrap_or_default();

        let b = create("Juegos", "win7", Arch::Win64).unwrap();
        assert!(calls().contains("wineboot --init") && calls().contains("winecfg -v win7"));
        assert!(calls().contains("RetinaMode /d n /f"));
        assert!(create("Juegos", "win7", Arch::Win64).is_err());

        let menu = b.drive_c().join("ProgramData/Microsoft/Windows/Start Menu/Programs/Steam");
        fs::create_dir_all(&menu).unwrap();
        fs::write(menu.join("Steam.lnk"), "").unwrap();
        fs::write(menu.join("Uninstall Steam.lnk"), "").unwrap();
        let apps = b.drive_c().join("Program Files (x86)/Steam/steamapps");
        fs::create_dir_all(&apps).unwrap();
        fs::write(apps.join("appmanifest_730.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"730\"\n\t\"name\"\t\t\"Counter-Strike 2\"\n}\n").unwrap();
        fs::write(apps.join("appmanifest_228980.acf"), "\t\"appid\" \"228980\"\n\t\"name\" \"Steamworks Common Redistributables\"\n").unwrap();
        let progs = programs(&b);
        assert_eq!(progs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Counter-Strike 2", "Steam"]);
        assert_eq!(progs[0].args, ["-applaunch", "730"]);
        assert_eq!(installed(&b).unwrap(), vec![("{ABC}".into(), "Test App".into())]);

        let (mut child, log) = launch(&b, &menu.join("Steam.lnk"), &["-silent".into()], None).unwrap();
        child.wait().unwrap();
        assert!(calls().contains("start /wait /unix") && calls().contains("Steam.lnk -silent"));
        assert!(log.exists() && !logs(&b).is_empty());

        snapshot(&b, "limpio").unwrap();
        fs::write(b.drive_c().join("roto.txt"), "x").unwrap();
        restore(&b, "limpio").unwrap();
        assert!(!b.drive_c().join("roto.txt").exists() && b.drive_c().exists());

        let copy = duplicate(&b, "Copia").unwrap();
        let renamed = rename(&copy, "Otra").unwrap();
        trash(&renamed).unwrap();
        assert_eq!(list().iter().map(|b| b.name.as_str()).collect::<Vec<_>>(), ["Juegos"]);

        let archive = tmp.join("juegos.tar.gz");
        export(&b, &archive).unwrap();
        assert_eq!(import_archive(&archive).unwrap().name, "Juegos 2");

        let cx = tmp.join("cx/Counter-Strike 2");
        fs::create_dir_all(cx.join("drive_c")).unwrap();
        fs::write(cx.join("cxbottle.conf"), CX).unwrap();
        let imported = import_crossover(&cx, "CS2").unwrap();
        assert_eq!(imported.config.graphics, Graphics::D3dmetal);
        assert!(cx.join("cxbottle.conf").exists()); // el original sigue ahí

        // Vinculado: corre con el `wine --bottle` de CrossOver y nunca se toca su carpeta.
        unsafe {
            std::env::set_var("BARRICA_CROSSOVER", tmp.join("cx"));
            std::env::set_var("BARRICA_CROSSOVER_BIN", &bin);
        }
        let linked = link_crossover("Counter-Strike 2", "CS2 vinculado").unwrap();

        // GPTK: copia tal cual (wine/ + external/), nvngx renombrado y licencia incluida.
        let gptk = tmp.join("Evaluation environment");
        for f in ["redist/lib/wine/x86_64-windows/d3d11.dll", "redist/lib/wine/x86_64-windows/nvngx-on-metalfx.dll",
            "redist/lib/wine/x86_64-unix/d3d11.so", "redist/lib/external/libd3dshared.dylib", "License.rtf"]
        {
            fs::create_dir_all(gptk.join(f).parent().unwrap()).unwrap();
            fs::write(gptk.join(f), f).unwrap();
        }
        assert!(install_gptk(&tmp).is_err());
        let layer = install_gptk(&gptk).unwrap();
        assert_eq!(layer, root().join("Layers/d3dmetal"));
        assert!(layer.join("wine/x86_64-windows/nvngx.dll").exists() && layer.join("License.rtf").exists());
        assert!(layer.join("wine/x86_64-unix/d3d11.so").exists() && layer.join("external/libd3dshared.dylib").exists());
        install_gptk(&gptk.join("redist")).unwrap(); // reinstalar reemplaza
        assert_eq!((linked.path.as_path(), linked.config.graphics), (cx.as_path(), Graphics::D3dmetal));
        let (mut child, _) = launch(&linked, &cx.join("drive_c/cs2.exe"), &[], None).unwrap();
        child.wait().unwrap();
        assert!(calls().contains("--bottle Counter-Strike 2 start /wait /unix"));
        assert!(duplicate(&linked, "x").is_err() && apply_settings(&linked).is_err());
        snapshot(&linked, "antes").unwrap();
        let renamed = rename(&linked, "CS2").unwrap_err(); // «CS2» ya existe
        assert!(renamed.contains("Ya existe"));
        trash(&linked).unwrap();
        assert!(cx.join("cxbottle.conf").exists() && get("CS2 vinculado").is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
