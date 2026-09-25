/// Analizador léxico de Forja.
///
/// Procesa texto de fuente y produce una lista de `Token`. Los comentarios
/// comienzan con `#` y llegan hasta el final de la línea. Los literales son
/// números decimales de rango `0..=255` (u8). `print_str` recibe una cadena
/// literal entre comillas dobles.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Ident,
    Number,
    String,
    U8,
    Var,
    While,
    If,
    Else,
    Print,
    PrintNum,
    PrintStr,
    Halt,
    True,
    False,
    Colon,
    Assign,
    Plus,
    Minus,
    And,
    Or,
    Xor,
    Lparen,
    Rparen,
    Lbrace,
    Rbrace,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Tok,
    pub text: String,
    pub line: usize,
    pub col: usize,
}

fn one(chars: &mut std::iter::Peekable<std::str::Chars>) {
    chars.next();
}

pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let mut toks = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1usize;
    let mut col = 1usize;

    while let Some(&c) = chars.peek() {
        match c {
            '#' => {
                while let Some(&c2) = chars.peek() {
                    if c2 == '\n' {
                        break;
                    }
                    one(&mut chars);
                    col += 1;
                }
            }
            '\n' => {
                one(&mut chars);
                line += 1;
                col = 1;
            }
            c if c.is_ascii_whitespace() => {
                one(&mut chars);
                col += 1;
            }
            c if c.is_ascii_digit() => {
                let start = col;
                let mut n = String::new();
                while let Some(&c2) = chars.peek() {
                    if c2.is_ascii_digit() {
                        n.push(c2);
                        one(&mut chars);
                        col += 1;
                    } else {
                        break;
                    }
                }
                let v: u16 = n
                    .parse()
                    .map_err(|_| format!("número inválido `{n}` (línea {line})"))?;
                if v > 255 {
                    return Err(format!(
                        "literal `{n}` fuera de rango (0..=255), línea {line}:{start}"
                    ));
                }
                toks.push(Token { kind: Tok::Number, text: n, line, col: start });
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = col;
                let mut id = String::new();
                while let Some(&c2) = chars.peek() {
                    if c2.is_ascii_alphanumeric() || c2 == '_' {
                        id.push(c2);
                        one(&mut chars);
                        col += 1;
                    } else {
                        break;
                    }
                }
                let kind = match id.as_str() {
                    "u8" => Tok::U8,
                    "var" => Tok::Var,
                    "while" => Tok::While,
                    "if" => Tok::If,
                    "else" => Tok::Else,
                    "print" => Tok::Print,
                    "print_num" => Tok::PrintNum,
                    "print_str" => Tok::PrintStr,
                    "halt" => Tok::Halt,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    _ => Tok::Ident,
                };
                toks.push(Token { kind, text: id, line, col: start });
            }
            '"' => {
                let start = col;
                one(&mut chars);
                col += 1;
                let mut s = String::new();
                loop {
                    match chars.peek() {
                        None => {
                            return Err(format!("cadena sin cerrar (línea {line}:{start})"))
                        }
                        Some('"') => {
                            one(&mut chars);
                            col += 1;
                            break;
                        }
                        Some('\\') => {
                            one(&mut chars);
                            col += 1;
                            match chars.next() {
                                Some('n') => s.push('\n'),
                                Some('t') => s.push('\t'),
                                Some('\\') => s.push('\\'),
                                Some('"') => s.push('"'),
                                Some(other) => {
                                    return Err(format!(
                                        "escape desconocido `\\{other}` (línea {line})"
                                    ))
                                }
                                None => {
                                    return Err(format!("cadena sin cerrar (línea {line}:{start})"))
                                }
                            }
                            col += 1;
                        }
                        Some(&c2) => {
                            s.push(c2);
                            one(&mut chars);
                            col += 1;
                        }
                    }
                }
                toks.push(Token { kind: Tok::String, text: s, line, col: start });
            }
            ':' => push(&mut chars, &mut toks, Tok::Colon, ":", &mut col, &mut line),
            '=' => {
                one(&mut chars);
                col += 1;
                if chars.peek() == Some(&'=') {
                    one(&mut chars);
                    col += 1;
                    toks.push(Token { kind: Tok::Eq, text: "==".into(), line, col: col - 2 });
                } else {
                    toks.push(Token { kind: Tok::Assign, text: "=".into(), line, col: col - 1 });
                }
            }
            '+' => push(&mut chars, &mut toks, Tok::Plus, "+", &mut col, &mut line),
            '-' => push(&mut chars, &mut toks, Tok::Minus, "-", &mut col, &mut line),
            '&' => push(&mut chars, &mut toks, Tok::And, "&", &mut col, &mut line),
            '^' => push(&mut chars, &mut toks, Tok::Xor, "^", &mut col, &mut line),
            '|' => push(&mut chars, &mut toks, Tok::Or, "|", &mut col, &mut line),
            '(' => push(&mut chars, &mut toks, Tok::Lparen, "(", &mut col, &mut line),
            ')' => push(&mut chars, &mut toks, Tok::Rparen, ")", &mut col, &mut line),
            '{' => push(&mut chars, &mut toks, Tok::Lbrace, "{", &mut col, &mut line),
            '}' => push(&mut chars, &mut toks, Tok::Rbrace, "}", &mut col, &mut line),
            '<' => {
                one(&mut chars);
                col += 1;
                if chars.peek() == Some(&'=') {
                    one(&mut chars);
                    col += 1;
                    toks.push(Token { kind: Tok::Le, text: "<=".into(), line, col: col - 2 });
                } else {
                    toks.push(Token { kind: Tok::Lt, text: "<".into(), line, col: col - 1 });
                }
            }
            '>' => {
                one(&mut chars);
                col += 1;
                if chars.peek() == Some(&'=') {
                    one(&mut chars);
                    col += 1;
                    toks.push(Token { kind: Tok::Ge, text: ">=".into(), line, col: col - 2 });
                } else {
                    toks.push(Token { kind: Tok::Gt, text: ">".into(), line, col: col - 1 });
                }
            }
            '!' => {
                one(&mut chars);
                col += 1;
                if chars.peek() == Some(&'=') {
                    one(&mut chars);
                    col += 1;
                    toks.push(Token { kind: Tok::Ne, text: "!=".into(), line, col: col - 2 });
                } else {
                    return Err(format!("carácter inesperado `!` (línea {line}:{col})"));
                }
            }
            other => {
                return Err(format!("carácter inesperado `{other}` (línea {line}:{col})"));
            }
        }
    }

    toks.push(Token { kind: Tok::Eof, text: String::new(), line, col });
    Ok(toks)
}

fn push(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    toks: &mut Vec<Token>,
    kind: Tok,
    text: &str,
    col: &mut usize,
    line: &mut usize,
) {
    for c in text.chars() {
        chars.next();
        *col += 1;
        if c == '\n' {
            *line += 1;
        }
    }
    toks.push(Token { kind, text: text.into(), line: *line, col: *col });
}