//! Pruebas de integración: compila programas Forja y los ejecuta en la VM
//! SILICON real (si el binario está disponible), comprobando la consola.

use forja::compile;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static UNIQ: AtomicUsize = AtomicUsize::new(0);

fn silicon_bin() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("SILICON") {
        return Some(std::path::PathBuf::from(p));
    }
    // El hermano del taller: 01-silicon (release).
    let sibling = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("01-silicon/target/release/silicon");
    sibling.exists().then_some(sibling)
}

fn run_program(src: &str) -> std::io::Result<String> {
    let asm = compile(src).expect("compilación forja");
    let dir = std::env::temp_dir().join("forja-e2e");
    std::fs::create_dir_all(&dir)?;
    let id = UNIQ.fetch_add(1, Ordering::SeqCst);
    let path = dir.join(format!("prog-{id}.s"));
    std::fs::write(&path, &asm)?;
    let sil = silicon_bin().expect("silicon no encontrado");
    let out = Command::new(sil)
        .args(["run", path.to_str().unwrap(), "--dump-console", "--max-cycles", "2000000"])
        .output()?;
    if !out.status.success() {
        panic!("silicon falló: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn cuenta_del_0_al_9() {
    let out = run_program(
        "var i:u8\ni = 0\nwhile i < 10 {\n print_num(i)\n print_str(\" \")\n i = i + 1\n}\nhalt()",
    )
    .unwrap();
    assert!(out.contains("0 1 2 3 4 5 6 7 8 9"), "got: {out}");
}

#[test]
fn numero_de_tres_cifras() {
    let out = run_program("var n:u8\nn = 255\nprint_num(n)\nhalt()").unwrap();
    assert!(out.contains("255"), "got: {out}");
}

#[test]
fn wraps_wrap_a_cero() {
    let out = run_program("var a:u8\na = 255\na = a + 1\nprint_num(a)\nhalt()").unwrap();
    assert!(out.contains("0"), "got: {out}");
}

#[test]
fn if_else_elige_la_rama_correcta() {
    let out = run_program(
        "var a:u8\na = 3\nif a < 5 {\n print_str(\"menor\")\n} else {\n print_str(\"mayor\")\n}\nhalt()",
    )
    .unwrap();
    assert!(out.contains("menor"), "got: {out}");
    assert!(!out.contains("mayor"));

    let out = run_program(
        "var a:u8\na = 9\nif a < 5 {\n print_str(\"menor\")\n} else {\n print_str(\"mayor\")\n}\nhalt()",
    )
    .unwrap();
    assert!(out.contains("mayor"), "got: {out}");
}

#[test]
fn comparaciones_complejas() {
    let out = run_program(
        "var i:u8\nvar j:u8\ni = 2\nj = 1\nif (i + 0) == (i + j - 1) {\n print_str(\"ok\")\n}\nhalt()",
    )
    .unwrap();
    assert!(out.contains("ok"), "got: {out}");
}

#[test]
fn todos_los_comparadores() {
    let src_tmpl = "var a:u8\nvar b:u8\na = {a}\nb = {b}\nif a {op} b {\n print_str(\"T\")\n} else {\n print_str(\"F\")\n}\nhalt()";
    // (a op b) → salida esperada, incluyendo casos límite.
    let cases: &[(&str, u8, u8, bool)] = &[
        ("==", 5, 5, true),
        ("==", 5, 6, false),
        ("!=", 5, 6, true),
        ("<", 4, 5, true),
        ("<", 5, 5, false),
        ("<=", 5, 5, true),
        ("<=", 6, 5, false),
        (">", 6, 5, true),
        (">", 5, 5, false),
        (">=", 5, 5, true),
        (">=", 4, 5, false),
    ];
    for (op, a, b, want) in cases {
        let src = src_tmpl
            .replace("{op}", op)
            .replace("{a}", &a.to_string())
            .replace("{b}", &b.to_string());
        let out = run_program(&src).unwrap();
        let expect = if *want { "T" } else { "F" };
        assert!(
            out.contains(expect),
            "a={a} {op} b={b} → se esperaba `{expect}` pero se obtuvo: {out}"
        );
    }
}