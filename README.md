# Forja

Un mini-compilador, escrito en Rust, que traduce un lenguaje de alto nivel de
tipo `u8` a **ensamblador SILICON**, la CPU de 8 bits construida desde cero en
<a href="https://github.com/ThorrijaDEV/silicon">silicon</a>. El código
generado se ensambla y se ejecuta en la máquina virtual real: cada `forja run`
corre el programa compilado sobre la CPU emulada y muestra la consola.

Un "yunque sobre el que se forja" el binario final, capa por capa:
lenguaje → AST → ensamblador → CPU.

## Qué hace

Toma un programa Forja como este:

```text
# Cuenta del 0 al 9 separando con espacios
var i:u8
i = 0
while i < 10 {
    print_num(i)
    print_str(" ")
    i = i + 1
}
halt()
```

y produce un `.s` SILICON que, compilado y ejecutado, imprime:

```text
0 1 2 3 4 5 6 7 8 9
```

## Características

- **Lenguaje mínimo de tipos `u8`** con `var`, asignación, `while`, `if/else`,
  `print` (carácter), `print_num` (decimal), `print_str` (cadena con escapes
  `\n`, `\t`, `\"`, `\\`) y `halt()`.
- **Aritmética que envuelve con módulo 256**, idéntica a la CPU SILICON
  (`255 + 1 == 0`).
- **Operadores**: `+ - & ^ |` y comparaciones `== != < <= > >=` (sin signo,
  tal y como compara la ISA).
- **Dobleces de constantes** en condiciones (`if 1 < 2` salta directo).
- **Generación de código de una sola pasada**:
  - variables residentes en memoria,
  - expresiones evaluadas sobre el acumulador A,
  - operandos anidados mediante `PSH/POP` y un temporal en RAM,
  - condiciones resueltas con la tabla de banderas `Z/C` de los `CMP`.
- **Subrutina `print_num` generada** que divide por 100/10 con restas
  repetidas y emite el ASCII de cada dígito.
- **`print_str`** como bucle con direccionamiento *absoluto indexado* (`addr,X`)
  sobre cadenas `.asciz`.
- **Cero dependencias**: única crate, sin librerías externas.

## Arquitectura

Tubería de compilación clásica en tres etapas:

```
fuente.fj ──► lexer ──► parser (AST) ──► codegen ──► salida.s
                 │           │                        │
            tokens       while/if/expr             ensamblador
                │           │                    SILICON (.s → .bin)
                └───────────┴──────────────────────────────┘
                                       │
                                    CPU de 8 bits
```

| Módulo              | Responsabilidad                                  |
|---------------------|--------------------------------------------------|
| `src/lexer.rs`      | Tokens, literales `0..=255`, comentarios `#`, cadenas con escapes |
| `src/parser.rs`     | Analizador *recursive descent* → `Program (Vec<Stmt>)` |
| `src/codegen.rs`    | Genera ensamblador SILICON (variables en RAM, ALU sobre A, saltos por banderas) |
| `src/main.rs`       | CLI `forja archivo.fj [-o salida.s] [--run]`, localiza `silicon` |
| `tests/e2e.rs`      | Compila y ejecuta programas completos en la VM SILICON real |

La tabla de banderas de los saltos se documenta en el propio `codegen.rs`:
`CMP` fija `Z` (igualdad) y `C` (sin borrow, es decir `A >= operand`) como la
instrucción `SUB` descartando el resultado, por lo que cada comparador se
reduce a una secuencia exacta de `JZ/JC`.

## Instalación

Forja sólo necesita Rust. El compilador localiza el emulador `silicon` por
orden de prioridad: variable `SILICON`, binario `silicon` en `PATH`, o el
proyecto hermano `01-silicon` del mismo padre.

```text
git clone https://github.com/ThorrijaDEV/forja.git
cd forja
cargo build --release
```

Si todavía no tienes el emulador:

```text
git clone https://github.com/ThorrijaDEV/silicon.git
cd silicon && cargo build --release
```

## Uso

```text
# Compilar a ensamblador
forja ejemplos/demo.fj -o demo.s

# Compilar y ejecutar en la VM SILICON
forja ejemplos/demo.fj --run

# Con SILICON a mano, tantas etapas como quieras
silicon asm demo.s -o demo.bin
silicon disasm demo.bin --base $8000
silicon run demo.s --dump-console --max-cycles 2000000
```

En la carpeta `ejemplos/` hay programas de referencia:

- `demo.fj` — cuenta del 0 al 9, imprime el máximo `u8` (255), `if/else`.
- `stress.fj` — expresión derecha anidada (PSH/POP + `_tmp`), condición
  compleja (`_cmp`), envueltas de módulo 256 y comparación con ambos lados
  complejos.

## Configuración

| Variable de entorno | Efecto                          |
|---------------------|---------------------------------|
| `SILICON`          | Ruta exacta del binario `silicon` |

## Testing

```text
cargo test
```

- **Pruebas unitarias** (léxico, sintaxis, reglas asociativas y de rango,
  forma del código generado, doblez de constantes).
- **Pruebas de integración** (`tests/e2e.rs`): compilan programas Forja y los
  ejecutan de verdad en la VM, comprobando la consola; incluyen una tabla que
  verifica los **seis comparadores** con sus casos límite (igualdad incluida).

## Limitaciones

- Tipos únicamente `u8`; sin funciones definidas por el usuario ni punteros.
- Operadores `* /` no implementados (hay que acumular con sumas).
- Las comparaciones sólo se admiten como condición de `while`/`if`, no dentro
  de expresiones.
- Aritmética sin signo en todas las comparaciones (igual que la ISA).
- El compilador no optimiza más allá del doblez de constantes en condiciones.

## Licencia

MIT — consulta el archivo `LICENSE`.