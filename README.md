# TP Final IntroPPL

Proyecto final de la materia **Introducción a Probabilistic Programming Languages (IntroPPL)**.
Este repositorio implementa un parser e intérprete para un lenguaje probabilístico inspirado en el libro *An Introduction to Probabilistic Programming*.

## Descripción

El proyecto está escrito en **Rust** y organiza el análisis del lenguaje dentro del módulo `parser`.

- `src/main.rs`: punto de entrada principal.
- `src/parser/mod.rs`: módulo raíz del parser.
- `src/parser/distribution.rs`: definiciones relacionadas con distribuciones probabilísticas.
- `src/parser/primitives.rs`: elementos primitivos del lenguaje.
- `src/parser/sexpr.rs`: manejo de expresiones S-expression.

## Objetivo

Crear un parser capaz de leer programas escritos en un PPL sencillo y un intérprete que evalúe esas expresiones con semántica probabilística.

El diseño está basado en los conceptos del libro:
- gramática y análisis sintáctico de expresiones probabilísticas
- modelos generativos con distribuciones
- inferencia básica en un lenguaje probabilístico

## Uso

Compilar y ejecutar el proyecto con Cargo:

```bash
cargo run
```

## Estructura de módulos

El módulo `parser` agrupa los componentes del análisis:

```rust
mod parser;

// uso de parser::distribution, parser::primitives, parser::sexpr
```

## Notas del proyecto

- El parser se implementa como un módulo con submódulos.
- El intérprete debe trabajar con los árboles sintácticos generados por el parser.
- El proyecto sirve como trabajo práctico final para demostrar comprensión de un PPL y su ejecución.

## Referencias

Basado en: *An Introduction to Probabilistic Programming*.
