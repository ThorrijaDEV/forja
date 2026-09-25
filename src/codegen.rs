/// Generador de código: traduce el AST de Forja a ensamblador SILICON.
///
/// Estrategia:
/// - Todas las variables viven en memoria (etiquetas `_v0.._vN`), puesto que
///   la CPU SILICON sólo tiene los registros A/B/X/Y y la aritmética trabaja
///   siempre sobre el acumulador A.
/// - Las expresiones se evalúan dejando el resultado en A. Un binario con
///   lado derecho simple (variable o literal) se emite en una sola
///   `ADD/SUB/AND/OR/XOR`; si el lado derecho es anidado se usa el temporal
///   `_tmp` con `PSH A / STA _tmp / POP A`.
/// - Las condiciones se resuelven con saltos. `CMP` fija banderas como `SUB`
///   descartando el resultado, por lo que una comparación `l <op> r` se
///   traduce a una secuencia `JZ/JC/JNC` (ver `branches_direct`). Cuando el
///   lado derecho es una expresión compleja se guarda el izquierdo en `_cmp`,
///   se evalúa el derecho en A y se usa la tabla invertida `branches_rev`.
/// - `print_num` es una subrutina generada que divide por 100/10 mediante
///   restas repetidas en RAM y escribe el ASCII de cada dígito.
/// - `print_str` emite un bucle con direccionamiento `addr,X` sobre una
///   cadena `.asciz` colocada en la zona de datos.
///
/// Todo el código compilado es de una sola pasada: las etiquetas de datos se
/// referencian hacia delante y el ensamblador SILICON las resuelve.

use crate::parser::{BinOp, CmpOp, Cond, Expr, Program, Stmt};

pub struct Codegen {
    out: String,
    labels: usize,
    vars: Vec<(String, String)>,
    strs: Vec<String>,
    used_str: bool,
    used_cmp: bool,
}

impl Default for Codegen {
    fn default() -> Self {
        Codegen {
            out: String::new(),
            labels: 0,
            vars: Vec::new(),
            strs: Vec::new(),
            used_str: false,
            used_cmp: false,
        }
    }
}

impl Codegen {
    fn emit(&mut self, s: impl AsRef<str>) {
        self.out.push_str(s.as_ref());
        self.out.push('\n');
    }
    fn label(&mut self, tag: &str) -> String {
        self.labels += 1;
        format!("_{}{}", tag, self.labels)
    }
    fn var_label(&self, name: &str) -> Result<String, String> {
        self.vars
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, l)| l.clone())
            .ok_or_else(|| format!("variable no declarada `{name}`"))
    }

    /// Compila un programa completo a texto ensamblador SILICON.
    pub fn compile(prog: &Program) -> Result<String, String> {
        let mut cg = Codegen::default();
        cg.emit(".origin $8000");
        cg.emit("; --- c\u{00f3}digo del programa ---");
        cg.stmts(&prog.stmts)?;
        cg.emit("    HLT");

        cg.emit("");
        cg.emit("; --- subrutina print_num: escribe A en decimal (0..=255) ---");
        cg.emit("print_num:");
        cg.emit("    STA _n1          ; valor de trabajo");
        cg.emit("    LDA #100");
        cg.emit("    STA _n2          ; divisor = 100");
        cg.emit("    LDA #0");
        cg.emit("    STA _n3          ; contador de centenas");
        cg.emit("_h_loop:");
        cg.emit("    LDA _n1");
        cg.emit("    CMP _n2");
        cg.emit("    JNC _h_done      ; _n1 < 100");
        cg.emit("    LDA _n1");
        cg.emit("    SUB _n2");
        cg.emit("    STA _n1");
        cg.emit("    LDA _n3");
        cg.emit("    INC A");
        cg.emit("    STA _n3");
        cg.emit("    JMP _h_loop");
        cg.emit("_h_done:");
        cg.emit("    LDA _n3");
        cg.emit("    JZ _t_phase");
        cg.emit("    ADD #$30         ; '0'");
        cg.emit("    OUT");
        cg.emit("_t_phase:");
        cg.emit("    LDA #10");
        cg.emit("    STA _n2          ; divisor = 10");
        cg.emit("    LDA #0");
        cg.emit("    STA _n4          ; contador de decenas");
        cg.emit("_t_loop:");
        cg.emit("    LDA _n1");
        cg.emit("    CMP _n2");
        cg.emit("    JNC _t_done      ; _n1 < 10");
        cg.emit("    LDA _n1");
        cg.emit("    SUB _n2");
        cg.emit("    STA _n1");
        cg.emit("    LDA _n4");
        cg.emit("    INC A");
        cg.emit("    STA _n4");
        cg.emit("    JMP _t_loop");
        cg.emit("_t_done:");
        cg.emit("    LDA _n3          ; ¿hay algo impreso ya?");
        cg.emit("    JNZ _print_tens");
        cg.emit("    LDA _n4");
        cg.emit("    JZ _units");
        cg.emit("_print_tens:");
        cg.emit("    LDA _n4");
        cg.emit("    ADD #$30");
        cg.emit("    OUT");
        cg.emit("_units:");
        cg.emit("    LDA _n1");
        cg.emit("    ADD #$30");
        cg.emit("    OUT");
        cg.emit("    RET");

        cg.data();
        Ok(cg.out)
    }

    fn data(&mut self) {
        self.emit("");
        self.emit("; --- zona de datos ---");
        let mut lines: Vec<String> = Vec::new();
        for (name, lbl) in self.vars.iter() {
            lines.push(format!("{lbl}: ; {name}"));
            lines.push("    .space 1".to_string());
        }
        if self.used_cmp {
            lines.push("_cmp: ; temporal de comparaciones".to_string());
            lines.push("    .space 1".to_string());
            lines.push("_tmp: ; temporal de expresiones".to_string());
            lines.push("    .space 1".to_string());
        }
        lines.push("_n1: ; print_num interno".to_string());
        lines.push("    .space 1".to_string());
        lines.push("_n2: ; print_num interno".to_string());
        lines.push("    .space 1".to_string());
        lines.push("_n3: ; print_num interno".to_string());
        lines.push("    .space 1".to_string());
        lines.push("_n4: ; print_num interno".to_string());
        lines.push("    .space 1".to_string());
        if self.used_str {
            for (i, s) in self.strs.iter().enumerate() {
                lines.push(format!("_str{i}:"));
                lines.push(format!("    .asciz \"{}\"", escape_asciz(s)));
            }
        }
        for l in lines {
            self.emit(l);
        }
    }

    fn stmts(&mut self, list: &[Stmt]) -> Result<(), String> {
        for s in list {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Var(name) => {
                let lbl = format!("_v{}", self.vars.len());
                self.emit(format!("    ; var {name} -> {lbl}"));
                self.vars.push((name.clone(), lbl));
            }
            Stmt::Assign(name, e) => {
                let lbl = self.var_label(name)?;
                self.expr_to_a(e)?;
                self.emit(format!("    STA {lbl}"));
            }
            Stmt::Print(e) => {
                self.expr_to_a(e)?;
                self.emit("    OUT");
            }
            Stmt::PrintNum(e) => {
                self.expr_to_a(e)?;
                self.emit("    CALL print_num");
            }
            Stmt::PrintStr(text) => {
                let id = self.strs.len();
                let slbl = format!("_str{id}");
                self.strs.push(text.clone());
                self.used_str = true;
                let loop_l = self.label("ps");
                let done_l = self.label("pse");
                self.emit("    LDX #0");
                self.emit(format!("{loop_l}:"));
                self.emit(format!("    LDA {slbl},X"));
                self.emit("    JZ ".to_owned() + &done_l);
                self.emit("    OUT");
                self.emit("    TXA");
                self.emit("    INC A");
                self.emit("    TAX");
                self.emit("    JMP ".to_owned() + &loop_l);
                self.emit(format!("{done_l}:"));
            }
            Stmt::Halt => {
                self.emit("    HLT");
            }
            Stmt::While(cond, body) => {
                let begin = self.label("w");
                let done = self.label("wd");
                self.emit(format!("{begin}:"));
                self.cond(cond, &done)?;
                self.stmts(body)?;
                self.emit("    JMP ".to_owned() + &begin);
                self.emit(format!("{done}:"));
            }
            Stmt::If(cond, then, els) => {
                let then_l = self.label("t");
                let done = self.label("e");
                if els.is_empty() {
                    self.cond_else(cond, &then_l, &done)?;
                    self.emit(format!("{then_l}:"));
                    self.stmts(then)?;
                    self.emit(format!("{done}:"));
                } else {
                    let else_l = self.label("f");
                    self.cond_else(cond, &then_l, &else_l)?;
                    self.emit(format!("{then_l}:"));
                    self.stmts(then)?;
                    self.emit("    JMP ".to_owned() + &done);
                    self.emit(format!("{else_l}:"));
                    self.stmts(els)?;
                    self.emit(format!("{done}:"));
                }
            }
        }
        Ok(())
    }

    /// Condición de un `while`/`if`: si es falsa salta a `done`; si es
    /// verdadera continúa con el siguiente bloque (el cuerpo).
    fn cond(&mut self, c: &Cond, done: &str) -> Result<(), String> {
        let cont = self.label("c");
        self.cond_branch(c, &cont, done)?;
        self.emit(format!("{cont}:"));
        Ok(())
    }

    /// Igual que `cond` pero expone `then`/`else` explícitos.
    fn cond_else(&mut self, c: &Cond, then_l: &str, else_l: &str) -> Result<(), String> {
        self.cond_branch(c, then_l, else_l)
    }

    fn cond_branch(&mut self, c: &Cond, t_lbl: &str, f_lbl: &str) -> Result<(), String> {
        match c {
            Cond::Expr(e) => {
                self.expr_to_a(e)?;
                self.emit(format!("    JNZ {t_lbl}"));
                self.emit(format!("    JMP {f_lbl}"));
            }
            Cond::Cmp(op, l, r) => {
                if let (Expr::Num(a), Expr::Num(b)) = (l, r) {
                    let truth = match op {
                        CmpOp::Eq => a == b,
                        CmpOp::Ne => a != b,
                        CmpOp::Lt => a < b,
                        CmpOp::Le => a <= b,
                        CmpOp::Gt => a > b,
                        CmpOp::Ge => a >= b,
                    };
                    self.emit(format!("    JMP {}", if truth { t_lbl } else { f_lbl }));
                    return Ok(());
                }
                match r {
                    Expr::Num(_) | Expr::Var(_) => {
                        self.expr_to_a(l)?;
                        let operand = self.operand(r)?;
                        self.emit(format!("    CMP {operand}"));
                        self.branches_direct(*op, t_lbl, f_lbl);
                    }
                    _ => {
                        self.used_cmp = true;
                        self.expr_to_a(l)?;
                        self.emit("    STA _cmp");
                        self.expr_to_a(r)?;
                        self.emit("    CMP _cmp");
                        self.branches_rev(*op, t_lbl, f_lbl);
                    }
                }
            }
        }
        Ok(())
    }

    /// Banderas tras `CMP m` con A = lado izquierdo (operando = derecho).
    ///
    /// El bloque "verdadero" está inmediatamente después, por lo que caerse
    /// hacia abajo equivale a tomar la rama verdadera; toda secuencia de
    /// saltos debe ser exhaustiva (cada estado de banderas salta al cuerpo o
    /// a `f_lbl`). Convenciones: `Z` ⇔ A==m; `C` ⇔ A>=m (sin borrow); `N` es
    /// el signo de (A-m).
    fn branches_direct(&mut self, op: CmpOp, t_lbl: &str, f_lbl: &str) {
        match op {
            CmpOp::Eq => {
                self.jz(t_lbl);
                self.jmp(f_lbl);
            }
            CmpOp::Ne => {
                self.jnz(t_lbl);
                self.jmp(f_lbl);
            }
            // l < r  ⇔  (l-r) presta  ⇔  C==0 (caída al cuerpo)
            CmpOp::Lt => {
                self.jc(f_lbl);
            }
            // l <= r ⇔ l<r (C==0, caída) o l==r (Z)
            CmpOp::Le => {
                self.jz(t_lbl);
                self.jc(f_lbl);
            }
            // l > r  ⇔ C==1 y Z==0
            CmpOp::Gt => {
                self.jz(f_lbl);
                self.jc(t_lbl);
                self.jmp(f_lbl);
            }
            // l >= r ⇔ C==1
            CmpOp::Ge => {
                self.jc(t_lbl);
                self.jmp(f_lbl);
            }
        }
    }

    /// Banderas tras `CMP _cmp` con A = lado derecho (operando = izquierdo),
    /// es decir, la comparación queda invertida; s = (derecho - izquierdo)
    /// y `C` ⇔ derecho>=izquierdo. Se traduce `l OP r` en términos de `s`.
    fn branches_rev(&mut self, op: CmpOp, t_lbl: &str, f_lbl: &str) {
        match op {
            CmpOp::Eq => {
                self.jz(t_lbl);
                self.jmp(f_lbl);
            }
            CmpOp::Ne => {
                self.jnz(t_lbl);
                self.jmp(f_lbl);
            }
            // l < r ⇔ s>0 ⇔ C==1 y Z==0
            CmpOp::Lt => {
                self.jz(f_lbl);
                self.jc(t_lbl);
                self.jmp(f_lbl);
            }
            // l <= r ⇔ r>=l ⇔ C==1
            CmpOp::Le => {
                self.jc(t_lbl);
                self.jmp(f_lbl);
            }
            // l > r ⇔ s<0 ⇔ C==0 (caída al cuerpo)
            CmpOp::Gt => {
                self.jc(f_lbl);
            }
            // l >= r ⇔ r<=l ⇔ s==0 (Z) o s<0 (C==0, caída)
            CmpOp::Ge => {
                self.jz(t_lbl);
                self.jc(f_lbl);
            }
        }
    }

    fn jz(&mut self, lbl: &str) {
        self.emit(format!("    JZ {lbl}"));
    }
    fn jnz(&mut self, lbl: &str) {
        self.emit(format!("    JNZ {lbl}"));
    }
    fn jc(&mut self, lbl: &str) {
        self.emit(format!("    JC {lbl}"));
    }
    fn jmp(&mut self, lbl: &str) {
        self.emit(format!("    JMP {lbl}"));
    }

    /// Evalúa una expresión dejando el resultado en A.
    fn expr_to_a(&mut self, e: &Expr) -> Result<(), String> {
        match e {
            Expr::Num(n) => {
                self.emit(format!("    LDA #${n:02X}"));
            }
            Expr::Var(name) => {
                let lbl = self.var_label(name)?;
                self.emit(format!("    LDA {lbl}"));
            }
            Expr::Bin(op, l, r) => {
                self.expr_to_a(l)?;
                let mnem = match op {
                    BinOp::Add => "ADD",
                    BinOp::Sub => "SUB",
                    BinOp::And => "AND",
                    BinOp::Or => "OR",
                    BinOp::Xor => "XOR",
                };
                match &**r {
                    Expr::Num(_) | Expr::Var(_) => {
                        let operand = self.operand(r)?;
                        self.emit(format!("    {mnem} {operand}"));
                    }
                    _ => {
                        self.used_cmp = true;
                        self.emit("    PSH A");
                        self.expr_to_a(r)?;
                        self.emit("    STA _tmp");
                        self.emit("    POP A");
                        self.emit(format!("    {mnem} _tmp"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Operando simple (literal hex o etiqueta) para ALU/CMP abs/imm.
    fn operand(&mut self, e: &Expr) -> Result<String, String> {
        match e {
            Expr::Num(n) => Ok(format!("#${n:02X}")),
            Expr::Var(name) => self.var_label(name),
            _ => Err("operando complejo no soportado aquí".into()),
        }
    }
}

/// Re-codifica el contenido de una cadena para el directivo `.asciz` del
/// ensamblador SILICON (que ahora entiende `\n`, `\t`, `\\` y `\"`).
fn escape_asciz(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn asm(src: &str) -> String {
        let p = Parser::parse(src).unwrap();
        Codegen::compile(&p).unwrap()
    }

    #[test]
    fn tiny_program_shape() {
        let a = asm("var x:u8\nx = 1\nhalt()");
        assert!(a.contains(".origin $8000"));
        assert!(a.contains("HLT"));
        assert!(a.contains("LDA #$01"));
        assert!(a.contains("STA _v0"));
        assert!(a.contains("_v0:"));
        assert!(a.contains(".space 1"));
    }

    #[test]
    fn rejects_undeclared_var() {
        let p = Parser::parse("y = 5").unwrap();
        assert!(Codegen::compile(&p).is_err());
    }

    #[test]
    fn while_loop_has_labels() {
        let a = asm("var i:u8\nwhile i < 3 {\n i = i + 1\n}");
        assert!(a.contains("_w1:"));
        assert!(a.contains("JMP _w1"));
        assert!(a.contains("CMP #$03"));
        assert!(!a.contains("_c1:_c1"));
    }

    #[test]
    fn if_else_generated() {
        let a = asm("var a:u8\nif a == 1 { print_str(\"si\") } else { print_num(a) }");
        assert!(a.contains("JZ _t1"));
        assert!(a.contains(".asciz \"si\""));
        assert!(a.contains("CALL print_num"));
    }

    #[test]
    fn const_folding_condition() {
        let a = asm("if 1 < 2 { halt() }");
        // 1 < 2 es verdadero: debe saltar directo al cuerpo.
        assert!(a.contains("JMP _t1"));
        assert!(!a.contains("CMP #$01"));
    }
}