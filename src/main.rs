mod catalog;
mod core;
mod gui;

use crate::catalog::Target;
use crate::core::{self as cx, Arch};
use std::path::Path;

const HELP: &str = "Barrica: bottles de Wine para macOS.

  barrica                                 abre la app
  barrica list                            bottles y su tamaño
  barrica create <nombre> [winver] [32]   p. ej. create Juegos win10
  barrica run <bottle> <archivo> [args]   .exe .msi .lnk .bat (trazas con BARRICA_DEBUG=+seh)
  barrica programs <bottle>               launchers del menú Inicio
  barrica tool <bottle> <herramienta>     winecfg regedit taskmgr control explorer uninstaller
  barrica installed <bottle>              software instalado
  barrica shell <bottle>                  shell con WINEPREFIX y wine en el PATH
  barrica kill <bottle> [--force]         cierra todo lo que corre en el bottle
  barrica set <bottle> <clave> <valor>    windows graphics msync high_resolution description engine env.X
  barrica rename|clone <bottle> <nuevo>
  barrica delete <bottle>                 a la Papelera
  barrica snapshot <bottle> <etiqueta>    copia instantánea (clon APFS)
  barrica snapshots <bottle>
  barrica restore <bottle> <etiqueta>
  barrica export <bottle> <archivo.tar.gz>
  barrica import <archivo>
  barrica crossover                       bottles de CrossOver que se pueden importar
  barrica link-crossover <bottle> [nombre]   usar un bottle de CrossOver con su motor (p. ej. CS2)
  barrica import-crossover <bottle> [nombre] copia independiente (clon APFS)
  barrica catalog [texto]                 apps que Barrica sabe instalar (se actualiza solo)
  barrica install <app> [bottle]          instala una app del catálogo (en un bottle nuevo si no existe)
  barrica winetricks <bottle> <verbos...> p. ej. vcrun2022 dotnet48
  barrica engines                         motores Wine detectados
  barrica engine <ruta a wine>            elegir motor";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.is_empty() { gui::run() } else { cli(&args) };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn cli(args: &[String]) -> cx::Result<()> {
    let a = |i: usize| args.get(i).map(String::as_str).ok_or_else(|| format!("Faltan argumentos.\n\n{HELP}"));
    let bottle = || cx::get(a(1)?);
    match a(0)? {
        "list" => {
            for b in cx::list() {
                let size = cx::size(&b.path).map(cx::human).unwrap_or_default();
                println!("{:<28} {:<6} {:<6} {size}", b.name, b.config.windows, format!("{:?}", b.config.arch));
            }
        }
        "create" => {
            let windows = args.get(2).map(String::as_str).unwrap_or("win10");
            let arch = if args.get(3).is_some_and(|s| s == "32") { Arch::Win32 } else { Arch::Win64 };
            println!("Creando «{}» ({windows})…", a(1)?);
            cx::create(a(1)?, windows, arch)?;
            println!("Listo.");
        }
        "run" => {
            let debug = std::env::var("BARRICA_DEBUG").ok();
            let (mut child, log) = cx::launch(&bottle()?, Path::new(a(2)?), &args[3..], debug.as_deref())?;
            println!("Log: {}", log.display());
            child.wait().map_err(|e| e.to_string())?;
        }
        "programs" => cx::programs(&bottle()?).iter().for_each(|p| println!("{}\t{} {}", p.name, p.path.display(), p.args.join(" "))),
        "tool" => drop(cx::tool(&bottle()?, a(2)?)?),
        "installed" => cx::installed(&bottle()?)?.iter().for_each(|(id, name)| println!("{name}\t{id}")),
        "shell" => cx::shell(&bottle()?)?,
        "kill" => cx::kill(&bottle()?, args.iter().any(|s| s == "--force"))?,
        "set" => {
            let mut b = bottle()?;
            if b.linked().is_some() && a(2)? != "description" {
                return Err("Los ajustes de un bottle vinculado se cambian en CrossOver.".into());
            }
            cx::set(&mut b.config, a(2)?, args.get(3).map(String::as_str).unwrap_or(""))?;
            b.save()?;
            if matches!(a(2)?, "windows" | "high_resolution") {
                cx::apply_settings(&b)?;
            }
        }
        "rename" => drop(cx::rename(&bottle()?, a(2)?)?),
        "clone" => drop(cx::duplicate(&bottle()?, a(2)?)?),
        "delete" => cx::trash(&bottle()?)?,
        "snapshot" => cx::snapshot(&bottle()?, a(2)?)?,
        "snapshots" => cx::snapshots(&bottle()?).iter().for_each(|s| println!("{s}")),
        "restore" => cx::restore(&bottle()?, a(2)?)?,
        "export" => cx::export(&bottle()?, Path::new(a(2)?))?,
        "import" => println!("Importado como «{}»", cx::import_archive(Path::new(a(1)?))?.name),
        "crossover" => cx::crossover_bottles().iter().for_each(|p| println!("{}", p.display())),
        "link-crossover" => {
            let name = args.get(2).map(String::as_str).unwrap_or(a(1)?);
            println!("Vinculado como «{}»", cx::link_crossover(a(1)?, name)?.name);
        }
        "import-crossover" => {
            let src = cx::crossover_dir().join(a(1)?);
            let name = args.get(2).map(String::as_str).unwrap_or(a(1)?);
            println!("Importado como «{}»", cx::import_crossover(&src, name)?.name);
        }
        "catalog" => {
            let c = catalog::refresh().unwrap_or_else(|_| catalog::load());
            for app in c.search(&args[1..].join(" "), None) {
                println!("{:<14} {:<26} {:<22} {}", app.id, app.name, c.category_name(&app.category), app.source());
            }
        }
        "install" => {
            let c = catalog::refresh().unwrap_or_else(|_| catalog::load());
            let id = a(1)?;
            let app = c.app(id).ok_or_else(|| format!("El catálogo no tiene «{id}». Mira `barrica catalog`."))?;
            let name = args.get(2).cloned().unwrap_or_else(|| app.name.clone());
            let target = if cx::get(&name).is_ok() { Target::Existing(name) } else { Target::New(name) };
            let last = std::cell::RefCell::new(String::new());
            let done = catalog::install(app, &target, &|step| {
                let label = step.label();
                let head = label.split('…').next().unwrap_or_default().to_string();
                if *last.borrow() != head {
                    println!("{label}");
                    *last.borrow_mut() = head;
                }
            })?;
            for w in &done.warnings {
                eprintln!("aviso: {w}");
            }
            let version = done.version.map(|v| format!(" {v}")).unwrap_or_default();
            println!("{}{version} instalado en «{}».", app.name, done.bottle);
        }
        "winetricks" => cx::winetricks(&bottle()?, &args[2..])?,
        "engines" => cx::engines().iter().for_each(|e| println!("{}\t{}", e.label, e.wine.display())),
        "engine" => cx::Settings { engine: Some(a(1)?.into()), ..cx::Settings::load() }.save()?,
        "help" | "-h" | "--help" => println!("{HELP}"),
        other => return Err(format!("Comando desconocido «{other}».\n\n{HELP}")),
    }
    Ok(())
}
