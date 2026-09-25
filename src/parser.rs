/// Analizador sintáctico (recursive descent) de Forja.
///
/// Gramática (los comentarios `#...` ya fueron eliminados por el lexer):
///
/// ```text
/// programa  := sentencia*
/// sentencia := 'var' identificador ':' 'u8'
///            | identificador '=' expr
///            | 'print' '(' expr ')'
///            | 'print_num' '(' expr ')'
///            | 'print_str' '(' STRING ')'
///            | 'halt' '(' ')'
///            | 'while' cond bloque
///            | 'if' cond bloque ('else' bloque)?
/// bloque    := '{' sentencia* '}'
/// cond      := expr | cmp (=='==','!=','<','<=','>','>=')
/// expr      := sumando (('+'|'-'|'&'|'^'|'|') sumando)*
/// sumando   := NUMERO | 'true' | 'false' | identificador | '(' expr ')'
/// ```
///
/// Todas las variables son `u8` y la aritmética envuelve con módulo 256, igual
/// que la CPU SILICON. Las comparaciones sólo se admiten en condiciones.

use crate::lexer::{lex, Tok, Token};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Num(u8),
    Var(String),
    Bin(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cond {
    Expr(Expr),
    Cmp(CmpOp, Expr, Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    Var(String),
    Assign(String, Expr),
    Print(Expr),
    PrintNum(Expr),
    PrintStr(String),
    Halt,
    While(Cond, Vec<Stmt>),
    If(Cond, Vec<Stmt>, Vec<Stmt>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }
    fn next(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn err(&self, msg: &str) -> String {
        let t = self.peek();
        format!("{msg} (línea {}:{})", t.line, t.col)
    }
    fn expect(&mut self, kind: Tok, what: &str) -> Result<Token, String> {
        let t = self.peek();
        if t.kind == kind {
            Ok(self.next())
        } else {
            Err(self.err(&format!("se esperaba {what}, se encontró `{}`", t.text)))
        }
    }
    fn eat(&mut self, kind: Tok) -> bool {
        if self.peek().kind == kind {
            self.next();
            true
        } else {
            false
        }
    }

    pub fn parse(src: &str) -> Result<Program, String> {
        let toks = lex(src)?;
        let mut p = Parser { toks, pos: 0 };
        let mut stmts = Vec::new();
        while p.peek().kind != Tok::Eof {
            stmts.push(p.stmt()?);
        }
        Ok(Program { stmts })
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        let t = self.peek().clone();
        match t.kind {
            Tok::Var => {
                self.next();
                let name = self.expect(Tok::Ident, "nombre de variable")?.text;
                self.expect(Tok::Colon, "`:`")?;
                self.expect(Tok::U8, "tipo `u8`")?;
                Ok(Stmt::Var(name))
            }
            Tok::Ident => {
                self.next();
                let name = t.text;
                self.expect(Tok::Assign, "`=`")?;
                let e = self.expr()?;
                Ok(Stmt::Assign(name, e))
            }
            Tok::Print => {
                self.next();
                self.expect(Tok::Lparen, "`(`")?;
                let e = self.expr()?;
                self.expect(Tok::Rparen, "`)`")?;
                Ok(Stmt::Print(e))
            }
            Tok::PrintNum => {
                self.next();
                self.expect(Tok::Lparen, "`(`")?;
                let e = self.expr()?;
                self.expect(Tok::Rparen, "`)`")?;
                Ok(Stmt::PrintNum(e))
            }
            Tok::PrintStr => {
                self.next();
                self.expect(Tok::Lparen, "`(`")?;
                let s = self.expect(Tok::String, "cadena literal")?.text;
                self.expect(Tok::Rparen, "`)`")?;
                Ok(Stmt::PrintStr(s))
            }
            Tok::Halt => {
                self.next();
                self.expect(Tok::Lparen, "`(`")?;
                self.expect(Tok::Rparen, "`)`")?;
                Ok(Stmt::Halt)
            }
            Tok::While => {
                self.next();
                let c = self.cond()?;
                let body = self.block()?;
                Ok(Stmt::While(c, body))
            }
            Tok::If => {
                self.next();
                let c = self.cond()?;
                let then = self.block()?;
                let mut els = Vec::new();
                if self.eat(Tok::Else) {
                    els = self.block()?;
                }
                Ok(Stmt::If(c, then, els))
            }
            _ => Err(self.err(&format!("sentencia inesperada `{}`", t.text))),
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Tok::Lbrace, "`{`")?;
        let mut v = Vec::new();
        while self.peek().kind != Tok::Rbrace {
            if self.peek().kind == Tok::Eof {
                return Err(self.err("falta `}` de cierre"));
            }
            v.push(self.stmt()?);
        }
        self.next();
        Ok(v)
    }

    fn cond(&mut self) -> Result<Cond, String> {
        let l = self.expr()?;
        let cmp_tok = self.peek().clone();
        let op = match cmp_tok.kind {
            Tok::Eq => CmpOp::Eq,
            Tok::Ne => CmpOp::Ne,
            Tok::Lt => CmpOp::Lt,
            Tok::Le => CmpOp::Le,
            Tok::Gt => CmpOp::Gt,
            Tok::Ge => CmpOp::Ge,
            _ => return Ok(Cond::Expr(l)),
        };
        self.next();
        let r = self.expr()?;
        Ok(Cond::Cmp(op, l, r))
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let mut left = self.additive()?;
        loop {
            let t = self.peek().clone();
            let op = match t.kind {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                Tok::And => BinOp::And,
                Tok::Or => BinOp::Or,
                Tok::Xor => BinOp::Xor,
                _ => return Ok(left),
            };
            self.next();
            let r = self.additive()?;
            left = Expr::Bin(op, Box::new(left), Box::new(r));
        }
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let t = self.peek().clone();
        match t.kind {
            Tok::Number => {
                self.next();
                Ok(Expr::Num(t.text.parse::<u8>().map_err(|_| self.err("número inválido"))?))
            }
            Tok::True => {
                self.next();
                Ok(Expr::Num(1))
            }
            Tok::False => {
                self.next();
                Ok(Expr::Num(0))
            }
            Tok::Ident => {
                self.next();
                Ok(Expr::Var(t.text))
            }
            Tok::Lparen => {
                self.next();
                let e = self.expr()?;
                self.expect(Tok::Rparen, "`)`")?;
                Ok(e)
            }
            _ => Err(self.err(&format!("expresión inesperada `{}`", t.text))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decl_assign_print() {
        let p = Parser::parse("var x:u8\nx = 1\nprint(x)\nhalt()").unwrap();
        assert_eq!(p.stmts.len(), 4);
        assert_eq!(p.stmts[0], Stmt::Var("x".into()));
        assert_eq!(p.stmts[1], Stmt::Assign("x".into(), Expr::Num(1)));
        assert_eq!(p.stmts[2], Stmt::Print(Expr::Var("x".into())));
        assert_eq!(p.stmts[3], Stmt::Halt);
    }

    #[test]
    fn parses_while_if_else() {
        let p = Parser::parse(
            "var a:u8\nwhile a < 10 {\n a = a + 1\n}\nif a == 10 { print_num(a) } else { print(a) }",
        )
        .unwrap();
        assert_eq!(p.stmts.len(), 3);
        let Stmt::While(cond, body) = &p.stmts[1] else {
            panic!("se esperaba while");
        };
        assert_eq!(
            *cond,
            Cond::Cmp(CmpOp::Lt, Expr::Var("a".into()), Expr::Num(10))
        );
        assert_eq!(body.len(), 1);
        let Stmt::If(_, then, els) = &p.stmts[2] else {
            panic!("se esperaba if");
        };
        assert_eq!(then.len(), 1);
        assert_eq!(els.len(), 1);
    }

    #[test]
    fn precedence_left_assoc() {
        let p = Parser::parse("var y:u8\ny = 1 + 2 + 3\n").unwrap();
        let Stmt::Assign(_, e) = &p.stmts[1] else {
            panic!("se esperaba asignación");
        };
        // ((1+2)+3)
        let Expr::Bin(BinOp::Add, l, r) = e else {
            panic!("se esperaba binario");
        };
        assert_eq!(**r, Expr::Num(3));
        let Expr::Bin(BinOp::Add, ll, lr) = &**l else {
            panic!("se esperaba binario anidado");
        };
        assert_eq!(Expr::Num(1), **ll);
        assert_eq!(Expr::Num(2), **lr);
    }

    #[test]
    fn literals_out_of_range_rejected() {
        assert!(Parser::parse("var a:u8\ny = 256\n").is_err());
        assert!(Parser::parse("y = 999\n").is_err());
    }

    #[test]
    fn comments_and_bool() {
        let p = Parser::parse("# comentario\nvar ok:u8\nok = true\nok = false\n").unwrap();
        assert_eq!(
            p.stmts[1],
            Stmt::Assign("ok".into(), Expr::Num(1))
        );
        assert_eq!(
            p.stmts[2],
            Stmt::Assign("ok".into(), Expr::Num(0))
        );
    }
}