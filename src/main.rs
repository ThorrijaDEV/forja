//! Interfaz de línea de comandos de Forja.
//!
//! ```text
//! forja hola.fj -o hola.s      # compila a ensamblador SILICON
//! forja hola.fj --run          # compila y ejecuta con silicon (si está)
//! ```

use forja::compile;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let mut args = env::args().skip(1);
    let file = match args.next() {
        Some(f) => f,
        None => {
            eprintln!("uso: forja <archivo.fj> [-o salida.s] [--run]");
            std::process::exit(2);
        }
    };
    let mut out_path = None;
    let mut run = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" => {
                out_path = Some(args.next().expect("falta ruta para -o"));
            }
            "--run" => run = true,
            other => {
                eprintln!("opción desconocida: {other}");
                std::process::exit(2);
            }
        }
    }

    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("forja: no se pudo leer `{file}`: {e}");
            std::process::exit(1);
        }
    };

    let asm = match compile(&src) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("forja: error en `{file}`: {e}");
            std::process::exit(1);
        }
    };

    let path = out_path.unwrap_or_else(|| {
        let p = Path::new(&file);
        format!("{}.s", p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(file.clone()))
    });
    if let Err(e) = std::fs::write(&path, &asm) {
        eprintln!("forja: no se pudo escribir `{path}`: {e}");
        std::process::exit(1);
    }
    println!("forja: ensamblador generado en {path}");

    if run {
        run_with_silicon(&path);
    }
}

fn run_with_silicon(asm_path: &str) {
    let binary = env::var("SILICON").ok().or_else(|| {
        // 1) PATH, 2) proyecto hermano del taller.
        if let Ok(_) = Command::new("silicon").arg("--version").stdout(Stdio::null()).status() {
            Some("silicon".to_string())
        } else {
            let sibling = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .map(|p| p.join("01-silicon/target/release/silicon"))
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().into_owned());
            sibling
        }
    });

    let Some(bin) = binary else {
        eprintln!("forja: silicon no está en PATH (define SILICON o compila el emulador); se omite la ejecución");
        return;
    };

    let out = Command::new(&bin)
        .args(["run", asm_path, "--dump-console", "--max-cycles", "20000000"])
        .output();
    match out {
        Ok(o) => {
            if !o.stdout.is_empty() {
                print!("{}", String::from_utf8_lossy(&o.stdout));
            }
            if !o.status.success() {
                eprintln!("forja: silicon terminó con error");
                eprint!("{}", String::from_utf8_lossy(&o.stderr));
            }
        }
        Err(e) => eprintln!("forja: no se pudo ejecutar {bin}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::io::Write;
    use std::path::Path;

    fn forja_bin() -> std::path::PathBuf {
        let target = env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"));
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        target.join(profile).join("forja")
    }

    #[test]
    fn cli_compiles_file() {
        let dir = std::env::temp_dir().join("forja-cli-test");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("hola.fj");
        std::fs::File::create(&src)
            .unwrap()
            .write_all(b"var x:u8\nx = 5\nprint_num(x)\nhalt()\n")
            .unwrap();
        let out = dir.join("hola.s");
        let status = std::process::Command::new(forja_bin())
            .arg(&src)
            .arg("-o")
            .arg(&out)
            .status()
            .unwrap();
        assert!(status.success());
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("CALL print_num"));
        assert!(text.contains(".origin $8000"));
        std::fs::remove_dir_all(&dir).ok();
    }
}