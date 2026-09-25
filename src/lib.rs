//! Forja — mini-compilador para la CPU SILICON.
//!
//! Traduce un pequeño lenguaje de alto nivel (`u8`, `while`, `if/else`,
//! `print`, `print_num`, `print_str`) a ensamblador SILICON, listo para
//! ensamblar y ejecutar en la máquina virtual de 8 bits.

pub mod codegen;
pub mod lexer;
pub mod parser;

pub use codegen::Codegen;
pub use lexer::lex;
pub use parser::{Parser, Program};

/// Compila texto fuente Forja a ensamblador SILICON.
pub fn compile(src: &str) -> Result<String, String> {
    let prog = Parser::parse(src)?;
    Codegen::compile(&prog)
}