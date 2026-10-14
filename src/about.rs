//! «Acerca de»: versión, motor, capas gráficas, sistema y buscador de versiones nuevas.

use crate::catalog;
use crate::core::{self as cx};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("BARRICA_COMMIT");
pub const BUILT: &str = env!("BARRICA_BUILT");
pub const REPO: &str = "ibearsmile2319/barrica";
pub const WEB: &str = "https://ibearsmile2319.github.io/barrica/";
pub const LICENSE: &str = "GPL-3.0-or-later";

pub fn repo_url() -> String {
    format!("https://github.com/{REPO}")
}

/// «0.2.0 (abc1234, 2026-10-14)».
pub fn version_line() -> String {
    let built = if BUILT.is_empty() { String::new() } else { format!(", {BUILT}") };
    format!("{VERSION} ({COMMIT}{built})")
}

/// Lo que se sabe del motor y del sistema. Tarda (lanza `wine --version`): en un hilo.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub engine: Option<String>,
    pub wine: Option<String>,
    /// Componentes del motor según `share/barrica/engine.toml`, que escribe `engine/build.sh`.
    pub parts: Vec<(String, String)>,
    pub d3dmetal: Option<String>,
    pub macos: String,
    pub chip: String,
    pub rosetta: bool,
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok().filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

pub fn gather() -> Info {
    let wine = cx::engine_for(None).ok();
    let engine = wine.as_ref().map(|w| {
        cx::engines().into_iter().find(|e| &e.wine == w).map_or_else(|| w.display().to_string(), |e| e.label)
    });
    let version = wine.as_ref().and_then(|w| run(&w.display().to_string(), &["--version"]));
    let parts = wine.as_deref().map(engine_parts).unwrap_or_default();
    Info {
        engine,
        wine: version,
        parts,
        d3dmetal: d3dmetal(),
        macos: run("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}")).unwrap_or_else(|| std::env::consts::OS.into()),
        chip: run("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|| std::env::consts::ARCH.into()),
        rosetta: Path::new("/Library/Apple/usr/share/rosetta/rosetta").exists(),
    }
}

/// `<motor>/share/barrica/engine.toml`: versiones de CrossOver, DXMT, MoltenVK… y parches aplicados.
fn engine_parts(wine: &Path) -> Vec<(String, String)> {
    let Some(root) = wine.parent().and_then(Path::parent) else { return vec![] };
    let Ok(text) = fs::read_to_string(root.join("share/barrica/engine.toml")) else { return vec![] };
    let Ok(map) = toml::from_str::<BTreeMap<String, toml::Value>>(&text) else { return vec![] };
    let label = |k: &str| match k {
        "crossover" => "Fuentes de CrossOver",
        "dxmt" => "DXMT",
        "moltenvk" => "MoltenVK",
        "mono" => "Wine Mono",
        "gecko" => "Wine Gecko",
        "patches" => "Parches propios",
        "built" => "Compilado",
        other => other,
    }
    .to_string();
    let order = ["crossover", "dxmt", "moltenvk", "mono", "gecko", "patches", "built"];
    let mut v: Vec<(usize, String, String)> = map
        .into_iter()
        .map(|(k, val)| {
            let text = match val {
                toml::Value::String(s) => s,
                toml::Value::Array(a) => a.iter().filter_map(|x| x.as_str()).map(|s| s.split('-').next().unwrap_or(s)).collect::<Vec<_>>().join(", "),
                other => other.to_string(),
            };
            (order.iter().position(|o| *o == k).unwrap_or(99), label(&k), text)
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, k, val)| (k, val)).collect()
}

/// D3DMetal (Game Porting Toolkit), si se instaló con `barrica gptk`, con su versión si la dice el framework.
fn d3dmetal() -> Option<String> {
    let layer = cx::root().join("Layers/d3dmetal");
    if !layer.exists() {
        return None;
    }
    let fw = layer.join("external/D3DMetal.framework");
    let version = [fw.join("Versions/A/Resources/Info.plist"), fw.join("Resources/Info.plist")]
        .iter()
        .find(|p| p.exists())
        .and_then(|p| run("plutil", &["-extract", "CFBundleShortVersionString", "raw", &p.display().to_string()]));
    Some(version.map_or_else(|| "instalado".into(), |v| format!("{v}, instalado")))
}

/// Texto para pegar en un reporte de error.
pub fn report(info: &Info) -> String {
    let mut s = format!("Barrica {}\n", version_line());
    let mut line = |k: &str, v: &str| s.push_str(&format!("{k}: {v}\n"));
    line("Motor", info.engine.as_deref().unwrap_or("ninguno"));
    if let Some(w) = &info.wine {
        line("Wine", w);
    }
    for (k, v) in &info.parts {
        line(k, v);
    }
    line("D3DMetal", info.d3dmetal.as_deref().unwrap_or("no instalado"));
    line("Sistema", &format!("{} · {} · Rosetta {}", info.macos, info.chip, if info.rosetta { "sí" } else { "no" }));
    let c = catalog::load();
    line("Catálogo", &format!("{} apps, del {}", c.apps.len(), c.updated));
    s
}

// ---------- Versiones nuevas (GitHub Releases) ----------

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub newer: bool,
}

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    html_url: String,
}

pub fn latest_release() -> cx::Result<Release> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "15", "-A", "Barrica", "-H", "Accept: application/vnd.github+json"])
        .arg(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("No se pudo consultar GitHub.".into());
    }
    let l: Latest = serde_json::from_slice(&out.stdout).map_err(|_| "Respuesta inesperada de GitHub.".to_string())?;
    let version = l.tag_name.trim_start_matches('v').to_string();
    let newer = catalog::cmp_version(&version, VERSION).is_gt();
    Ok(Release { version, url: l.html_url, newer })
}

/// Avisos de licencias de terceros: los que van dentro de la app o, si no, los del repositorio.
pub fn notices() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.join("Resources/THIRD_PARTY_NOTICES.md")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(format!("{}/blob/master/THIRD_PARTY_NOTICES.md", repo_url())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_engine_manifest() {
        assert!(version_line().starts_with(VERSION));
        let tmp = std::env::temp_dir().join(format!("barrica-about-{}", std::process::id()));
        fs::create_dir_all(tmp.join("bin")).unwrap();
        fs::create_dir_all(tmp.join("share/barrica")).unwrap();
        fs::write(
            tmp.join("share/barrica/engine.toml"),
            "moltenvk = \"v1.4.2\"\ncrossover = \"26.3.0\"\npatches = [\"0001-coreaudio\", \"0002-steam\"]\n",
        )
        .unwrap();
        let parts = engine_parts(&tmp.join("bin/wine"));
        assert_eq!(parts[0], ("Fuentes de CrossOver".into(), "26.3.0".into()));
        assert_eq!(parts[2], ("Parches propios".into(), "0001, 0002".into()));
        let _ = fs::remove_dir_all(&tmp);
    }
}
