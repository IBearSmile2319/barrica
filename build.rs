//! Versión exacta para «Acerca de»: commit y fecha de compilación.
use std::process::Command;

fn main() {
    let run = |cmd: &str, args: &[&str]| {
        Command::new(cmd).args(args).output().ok().filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let commit = run("git", &["rev-parse", "--short=7", "HEAD"])
        .or_else(|| std::env::var("GITHUB_SHA").ok().map(|s| s.chars().take(7).collect()))
        .unwrap_or_else(|| "local".into());
    let date = run("date", &["-u", "+%Y-%m-%d"]).unwrap_or_default();
    println!("cargo:rustc-env=BARRICA_COMMIT={commit}");
    println!("cargo:rustc-env=BARRICA_BUILT={date}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}
