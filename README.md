# Trabajo Final Introducción a los Lenguajes de Programación Probabilísticos

Trabajo final de **Martín Nievas Wilberger** para la materia Introducción a los Lenguajes de Programación Probabilísticos

# Acerca del Proyecto

El objetivo principal de este trabajo es el diseño e implementación desde cero de un **Lenguaje de Programación Probabilística de Orden Superior (HOPPL)**. El desarrollo toma como especificación formal y semántica el marco teórico presentado en el libro y paper de referencia *"An Introduction to Probabilistic Programming"* de **Jan-Willem van de Meent, Brooks Paige, Hongseok Yang y Frank Wood**.

### Requerimientos y Alcance

De acuerdo con la consigna, el proyecto cumple con dos núcleos de exigencia principales:

1. **Capacidades del Lenguaje:** El lenguaje diseñado no se limita a modelos estáticos de primer orden; incluye soporte nativo para construcciones avanzadas que habilitan grafos de computación dinámicos y de longitud variable:
   * **Clausuras (Closures):** Funciones de primera clase con captura de entorno léxico.
   * **Recursión:** Permite la definición de funciones recursivas, fundamentales para modelar procesos estocásticos complejos (como distribuciones geométricas o árboles probabilísticos).

2. **Motores de Inferencia:**
   Se solicita la implementación de los tres algoritmos de inferencia más fundamentales y comunes dentro del paradigma probabilístico para aproximar las distribuciones a posteriori:
   * **Likelihood Weighting** (Ponderación por Verosimilitud)
   * **Sequential Monte Carlo (SMC)** (Monte Carlo Secuencial / Filtro de Partículas)
   * **Single-Site Metropolis-Hastings (MH)** (Metropolis-Hastings de Sitio Único)

*Nota sobre la tecnología:* La consigna permitía la libre elección del lenguaje de programación para el desarrollo del intérprete. Los detalles técnicos, las justificaciones arquitectónicas y los beneficios de la tecnología seleccionada para este proyecto se detallan en la siguiente sección.

# Ejecutar Proyecto

En esta sección vamos a detallar todas las dependencias del proyecto y como correr el mismo.

## Dependencias

Como se mencionó anteriormente vamos a especificar las dependencias para utilizar el lenguaje, y aquí radica la magia de Rust y más específicamente en su Build System and Package Manager **Cargo**, el cual se instala de la siguiente manera:

### Instalación de Rustup

**Rustup** es el instalador y gestor de toolchains oficial de Rust. Se encarga de instalar `rustc` (el compilador), `cargo` (el build system y package manager) y mantenerlos actualizados.

#### macOS / Linux

Abrí una terminal y corré:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Esto descarga el script oficial de rustup y lo ejecuta. Te va a preguntar qué tipo de instalación querés (la opción `1) Proceed with installation (default)` alcanza para la mayoría de los casos).

Una vez terminado, cargá las variables de entorno en la sesión actual (o simplemente abrí una terminal nueva):

```bash
source "$HOME/.cargo/env"
```

En macOS, si no tenés instaladas las herramientas de línea de comandos de Xcode (necesarias para compilar), rustup te va a pedir que las instales. Podés adelantarte corriendo:

```bash
xcode-select --install
```

#### Windows

En Windows hay dos caminos:

1. **Instalador gráfico (recomendado):** descargar y ejecutar [`rustup-init.exe`](https://rustup.rs) desde el sitio oficial. El instalador va a detectar automáticamente si te falta el **Visual Studio C++ Build Tools** (requerido para linkear en Windows) y te va a ofrecer instalarlo.

2. **Con winget (PowerShell):**

   ```powershell
   winget install Rustlang.Rustup
   ```

En ambos casos, después de instalar hay que **reiniciar la terminal** para que las variables de entorno (`PATH`) se actualicen correctamente.

#### Verificar la instalación

En cualquiera de los tres sistemas, una vez instalado, verificá con:

```bash
rustc --version
cargo --version
```

Deberían aparecer las versiones instaladas (por ejemplo `rustc 1.8x.x` y `cargo 1.8x.x`). Si aparece "command not found", probablemente falte reiniciar la terminal o recargar el `PATH`.

## Compilar el proyecto

Una vez clonado el repositorio, parate en la carpeta raíz del proyecto (donde está el `Cargo.toml`) y corré:

```bash
cargo build --release
```

Esto descarga automáticamente todas las dependencias (crates) declaradas en `Cargo.toml` y compila el proyecto en modo optimizado. La primera compilación puede tardar unos minutos porque baja e instala todas las dependencias; las siguientes son incrementales y mucho más rápidas.

> Si solo querés compilar rápido para desarrollo (sin optimizaciones), podés usar `cargo build` a secas.

## Correr el proyecto

El binario soporta cinco modos de uso distintos:

### 1. Correr todas las demos hardcodeadas

```bash
cargo run
```

Ejecuta, en orden, las 7 demostraciones incluidas en el proyecto (Likelihood Weighting, SMC, seguridad estática de SMC, Single-Site MH, BBVI, Exact Enumeration y pruebas con factor), pausando entre cada una para que puedas leer los resultados antes de continuar.

### 2. Correr una demo específica

```bash
cargo run -- <numero>
```

Donde `<numero>` es un valor entre `1` y `7`. Por ejemplo:

```bash
cargo run -- 4
```

corre únicamente la demo de Single-Site Metropolis-Hastings, sin pausas ni el resto de las demos.

### 3. Correr un modelo `.hoppl` propio

```bash
cargo run -- <archivo.hoppl> <algoritmo>
```

Donde:

- `<archivo.hoppl>` es la ruta a un archivo de texto con un programa escrito en HOPPL.
- `<algoritmo>` es el motor de inferencia a usar. Los valores soportados son:

| Algoritmo | Valor a pasar |
|---|---|
| Likelihood Weighting | `lw` |
| Single-Site Metropolis-Hastings | `ssmh` |
| Sequential Monte Carlo | `smc` |
| Black-Box Variational Inference | `bbvi` |
| Exact Enumeration | `exact-enumeration` (alias: `enum`, `exact`) |

Por ejemplo:

```bash
cargo run -- modelos/mi_modelo.hoppl smc
```

### 4. Correr un modelo `.hoppl` propio determinisitico

```bash
cargo run -- <archivo.hoppl>
```

Donde:

- `<archivo.hoppl>` es la ruta a un archivo de texto con un programa determinisitico escrito en HOPPL.

### 5. Depurar un modelo `.hoppl` paso a paso (modo debug)

```bash
cargo run -- debug <archivo.hoppl> <algoritmo>
```

Levanta una interfaz de terminal (TUI, con `ratatui`) que permite ejecutar el programa paso a paso en vez de correrlo de punta a punta. El `<algoritmo>` no es solo cosmético: cada uno de los 5 motores de inferencia tiene su propia noción de "paso", fiel a su algoritmo real (ver [Debugger Interactivo de Inferencia](#debugger-interactivo-de-inferencia-tui) en Extras). Por ejemplo:

```bash
cargo run -- debug modelos/mi_modelo.hoppl ssmh
```

`programs/` incluye modelos pensados específicamente para probar el modo debug con cada algoritmo: `coin_bias.hoppl` (SMC/LW, varios `observe` sincronizados), `line_fit.hoppl` (SSMH/BBVI, dos latentes continuas), `three_coins.hoppl` (Exact Enumeration, árbol de 8 ramas) y `multi_factor.hoppl` (dos `factor` consecutivos). Cada uno trae en un comentario el comando exacto para correrlo.

## Correr los tests

El proyecto incluye tests automatizados para validar el parser, el intérprete, los distintos algoritmos de inferencia y tambien primitivas y distribuciones soportadas. Para correr **toda** la suite de tests:

```bash
cargo test
```

Si querés correr únicamente los tests de un módulo o archivo puntual (por ejemplo, solo los del parser o solo los de un algoritmo de inferencia en particular), pasale un filtro por nombre:

```bash
cargo test <nombre_del_test_o_modulo>
```

Cargo va a correr solo aquellos tests cuyo nombre (o el path del módulo que los contiene) coincida con el filtro dado. Por ejemplo:

```bash
cargo test tests_parser
```

correría solo los tests relacionados a validacion de AST construido y tokenización.

> Tip: agregá el flag `-- --nocapture` si querés ver los `println!` que hagan los tests mientras corren (por defecto Cargo oculta la salida estándar de los tests que pasan):
>
> ```bash
> cargo test -- --nocapture
> ```

# Acerca del Lenguaje

Este lenguaje, fuertemente inspirado en Lisp y Clojure, utiliza una sintaxis basada en **Expresiones S (S-expressions)**, donde el código y los datos comparten la misma estructura de listas anidadas. Todo el código es evaluado por la máquina virtual CEK, que interpreta estas listas para controlar el flujo, manejar variables y disparar efectos probabilísticos.

## Sintaxis

### Tipos de Datos Primitivos

* **Símbolos (Identificadores):** Secuencias de caracteres usadas para variables y funciones. Ej: `x`, `+`, `mi-variable`.

* **Números:** * Enteros de 64 bits. Ej: `42`, `-10`.
  * Flotantes de 64 bits. Ej: `3.14`, `-0.5`.

* **Booleanos:** `true` y `false`.

* **Cadenas de Texto:** Encerradas entre comillas dobles, con soporte para caracteres de escape. Ej: `"Hola Mundo\n"`.

* **Nulo:** Representa la ausencia de valor. Se escribe como `nil`.

### Estructuras y Formas Especiales

* **Listas:** Agrupan expresiones. El primer elemento de una lista se evalúa como la función u operador, y los demás como sus argumentos. Ej: `(+ 1 2)`.

* **Corchetes:** Sintácticamente idénticos a los paréntesis, se usan por convención para mejorar la legibilidad en definiciones de parámetros y variables. Ej: `[x 1 y 2]` es equivalente a `(x 1 y 2)`.

* **Comentarios:** Inician con punto y coma (`;`) y se extienden hasta el final de la línea.

* **Control de Flujo y Variables:**

  * `(let [var1 expr1 var2 expr2] cuerpo)`: Enlaza variables locales secuencialmente.

  * `(if condicion rama-verdadera rama-falsa)`: Bifurcación condicional.

  * `(fn [arg1 arg2] cuerpo)`: Define funciones anónimas (closures).

* **Efectos Probabilísticos:**

  * `(sample distribucion)`: Extrae un valor aleatorio de una distribución. Devuelve el control al motor de inferencia.

  * `(observe distribucion valor)`: Condiciona el modelo probabilístico observando que una distribución generó un valor específico, ajustando los pesos de la traza.

## Operaciones Soportadas

El entorno global provee un amplio conjunto de primitivas determinísticas para operar sobre los datos, funciones para manipular estructuras de datos y distribuciones para las sentencias `sample` y `observe`. Para todas las operaciones determinísticas están definidas en `src/parser/primitives.rs` en un HashMap donde la clave representa el símbolo y el valor es la función correspondiente en código Rust (También se incluyen aquí mismo las distribuciones).

### Aritmética y Matemáticas

* Básicas: `+`, `-`, `*`, `/`, `mod`

* Avanzadas: `sqrt`, `exp`, `log`, `pow`, `abs`, `floor`, `ceil`, `tanh`, `max`, `min`

### Lógica y Comparación

* Igualdad: `=`, `==`, `!=`

* Relacionales: `<`, `>`, `<=`, `>=`

* Booleanas: `and`, `or`, `not`

### Estructuras de Datos (Listas y Mapas)

* Creación: `vector` (o `list`), `hash-map`, `range`

* Acceso: `get`, `first`, `second`, `last`, `nth`, `peek`

* Modificación: `put` (o `assoc`), `rest`, `conj`, `cons`, `append`, `concat`

* Utilidad: `count`, `empty?`

* Predicados de tipo: `vector?`, `map?`, `number?`

### Operaciones Matriciales

Soporte nativo para álgebra lineal bidimensional (fundamental para modelos de Machine Learning y Redes Neuronales Bayesianas):

* `mat-mul`: Multiplicación de matrices (producto punto).

* `mat-add`: Suma matricial.

* `mat-transpose`: Transposición de matrices.

* `mat-tanh`, `mat-relu`: Funciones de activación aplicadas elemento a elemento.

* `mat-repmat`: Repetición (tiling) de matrices, equivalente a `np.tile`.

## Distribuciones Soportadas

El lenguaje soporta la instanciación de variables aleatorias a través de diversas familias de distribuciones paramétricas. Todas las distribuciones implementan métodos internos para muestrear (`sample`) o evaluar log-densidades (`log_prob`). Las mismas están definidas en el módulo `src/parser/distribution.rs`

### Distribuciones Continuas

* `(normal mu sigma)`: Distribución Normal (Gaussiana).

* `(log-normal mu sigma)`: Distribución Log-Normal.

* `(uniform a b)` / `(uniform-continuous a b)`: Distribución Uniforme continua en el intervalo $[a, b]$.

* `(exponential rate)`: Distribución Exponencial.

* `(beta alpha beta)`: Distribución Beta.

* `(gamma shape rate)`: Distribución Gamma.

* `(dirichlet [alphas...])`: Distribución Dirichlet (multivariada).

### Distribuciones Discretas

* `(bernoulli p)` / `(flip p)`: Ensayo de Bernoulli (moneda sesgada).

* `(poisson lam)`: Distribución de Poisson.

* `(discrete [probs...])` / `(categorical [probs...])`: Distribución Categórica dada una lista de probabilidades (se normalizan automáticamente).

* `(uniform-discrete lo hi)`: Distribución Uniforme discreta en el intervalo $[lo, hi)$.

# Tutorial: Escribiendo tu primer programa en HOPPL

Esta sección es una guía práctica y progresiva para aprender a escribir programas en HOPPL, partiendo de expresiones simples hasta llegar a un modelo probabilístico completo. Todos los ejemplos se pueden probar directamente pegándolos en un archivo `.hoppl` y corriendo `cargo run -- archivo.hoppl <algoritmo>` (ver la sección [Ejecutar Proyecto](#ejecutar-proyecto)).

### 1. Expresiones y aritmética

Como en todo Lisp, el primer elemento de una lista es el operador y el resto son sus argumentos:

```clojure
(+ 1 2)              ; -> 3
(* 3 (+ 1 1))         ; -> 6
(> 5 3)               ; -> true
```

### 2. Variables con `let`

`let` enlaza una o más variables locales, en orden, y evalúa un cuerpo final usándolas:

```clojure
(let [x 5
      y (+ x 2)]
  (* x y))            ; -> 35
```

Notá que `y` puede usar `x` porque `let` enlaza sus variables de forma secuencial (como un `let*` de Lisp), no en simultáneo.

### 3. Condicionales con `if`

```clojure
(let [x 10]
  (if (> x 5)
      "grande"
      "chico"))       ; -> "grande"
```

### 4. Funciones con `fn`

`fn` define una función anónima (closure). Podés asignarla a un nombre con `let` para reutilizarla:

```clojure
(let [cuadrado (fn [x] (* x x))]
  (cuadrado 4))       ; -> 16
```

El lenguaje también soporta recursión, pero con un matiz importante: `let` no es un `letrec`. Cuando `(fn [...] cuerpo)` se evalúa, la closure captura el entorno tal como está en ese instante, que es *antes* de que `let` termine de enlazar el nombre de la función. Por eso, una función no puede llamarse a sí misma simplemente por su propio nombre dentro de un `let`.

La forma estándar de lograr recursión en este caso es el truco clásico de **auto-aplicación**: la función recibe una copia de sí misma como argumento explícito, y se la vuelve a pasar en cada llamada recursiva. Con eso podemos escribir, por ejemplo, una distribución geométrica implementada recursivamente:

```clojure
; Cuenta cuántos "fracasos" (bernoulli p = false) ocurren antes del primer "éxito".
; Esto es exactamente la definición de una distribución Geométrica(p).
(let [geometrica
        (fn [self]
          (fn [p]
            (if (sample (bernoulli p))
                0
                (+ 1 ((self self) p)))))]
  ((geometrica geometrica) 0.3))
```

`(geometrica geometrica)` se aplica a sí misma para producir la función real de un argumento (`fn [p] ...`), ya con `self` correctamente enlazado en su entorno porque, a diferencia del nombre en `let`, `self` es un parámetro de función y sí se resuelve normalmente en el momento de la llamada. Dentro del cuerpo, `((self self) p)` repite el mismo truco para la llamada recursiva.

Esto también muestra algo importante del lenguaje: como `sample` puede devolver un valor distinto cada vez que se evalúa, la cantidad de veces que `geometrica` se llama a sí misma varía en cada ejecución. Es un ejemplo simple de grafo de computación de **longitud variable**, uno de los requisitos centrales de un HOPPL.

### 5. Efectos probabilísticos: `sample` y `observe`

* `(sample dist)` extrae un valor aleatorio de una distribución.
* `(observe dist valor)` le dice al motor de inferencia "asumí que `dist` generó exactamente `valor`", condicionando el modelo.

```clojure
; mu es una variable aleatoria latente con prior Normal(0, 1)
(let [mu (sample (normal 0 1))]
  ; Observamos que, bajo Normal(mu, 1), el valor generado fue 3.0
  (observe (normal mu 1) 3.0)
  mu)
```

Este programa por sí solo no "hace" nada determinístico: define un modelo probabilístico. Necesita un motor de inferencia (`lw`, `ssmh`, `smc`, `bbvi` o `exact-enumeration`) para aproximar la distribución a posteriori de `mu` dado que observamos 3.0.

### 6. Un modelo completo: moneda sesgada (Beta-Bernoulli)

Juntando todo lo anterior, así se ve un modelo bayesiano clásico completo: queremos inferir el sesgo `p` de una moneda a partir de haber observado 3 caras y 1 cruz.

```clojure
; Prior: p ~ Beta(2, 2)
; Likelihood: observamos 3 caras (true) y 1 cruz (false)
(let [p (sample (beta 2.0 2.0))]
    (observe (bernoulli p) true)
    (observe (bernoulli p) true)
    (observe (bernoulli p) true)
    (observe (bernoulli p) false)
    p)
```

Guardá esto en `moneda.hoppl` y corré, por ejemplo:

```bash
cargo run -- moneda.hoppl smc
```

para aproximar la distribución a posteriori de `p` usando Sequential Monte Carlo. Podés cambiar `smc` por `lw`, `ssmh` o `bbvi` para comparar cómo cada motor de inferencia resuelve el mismo modelo.

### 7. Condicionamiento suave con `factor`

`observe` es en realidad un caso particular de una operación más general: sumar densidad de log-verosimilitud a la traza de ejecución. `observe` lo hace de forma indirecta, le pasás una distribución y un valor, y el motor calcula `log_prob(valor)` por vos. El operador `(factor <expr>)` te da acceso directo a ese mecanismo: suma el número que le pases, tal cual, al log-peso acumulado de la traza, sin necesidad de una distribución ni de un valor observado concreto.

```clojure
(factor <expr>)
```

Esto es útil cuando lo que querés modelar no es "observé exactamente este valor" sino una noción más flexible de "esta configuración es más o menos plausible". Por ejemplo, podés reescribir a mano la densidad gaussiana que usaría `observe` internamente:

```clojure
; Equivalente (salvo la constante de normalización) a:
;   (observe (normal mu 1.0) 3.0)
(let [mu (sample (normal 0.0 10.0))
      diff (- mu 3.0)
      log_lik (* -0.5 (* diff diff))]
    (factor log_lik)
    mu)
```

La diferencia clave con `observe` es que `factor` no compara contra un dato exacto: te obliga a escribir vos mismo la función de densidad (o cualquier otra función de "qué tan bueno es este estado"), en vez de delegarla en una distribución con nombre. Esto habilita modelos donde la evidencia no es un punto fijo sino una preferencia continua, por ejemplo, penalizar configuraciones alejadas de un valor deseado sin fijar ese valor como una observación puntual:

```clojure
; Preferimos que p este cerca de 0.5, sin observar ningun dato concreto.
(let [p (sample (beta 2.0 2.0))
      penalizacion (* -2.0 (* (- p 0.5) (- p 0.5)))]
    (factor penalizacion)
    p)
```

**Importante:** a diferencia de `sample`, `factor` no le devuelve el control al motor de inferencia: no hay ninguna decisión estocástica que tomar, solo un número que sumar. Por eso podés usarlo con cualquiera de los algoritmos de inferencia soportados (`lw`, `ssmh`, `smc`) sin que la máquina se pause en ese punto. Como valor de retorno de la expresión, `(factor <expr>)` siempre produce `nil`, así que en general se usa como una sentencia intermedia dentro de un `let`, no como el valor final de un cuerpo.

### 8. Próximos pasos

A partir de acá, las secciones **Acerca del Lenguaje** (arriba) y **Extras** (más abajo) documentan todas las primitivas, distribuciones y garantías de seguridad estática (por ejemplo, qué patrones evita el analizador de SMC) que vas a necesitar para escribir modelos más complejos.

# Estructura del Proyecto

El código fuente está organizado de forma modular siguiendo las convenciones e idiomáticas de **Rust**, separando claramente las etapas de análisis sintáctico (frontend), evaluación y ejecución (backend), los motores de inferencia matemática y la batería de pruebas:

```plaintext
TP-FINAL-PPL
|-- Cargo.lock
|-- Cargo.toml                  -> Configuración y dependencias en Rust
|-- LICENSE                     -> Licencia del proyecto
|-- README.md                   -> Documentación principal del proyecto
|-- programs/                   -> Directorio de programas hoppl
|-- src/
|   |-- main.rs                 -> Punto de entrada y ejecutable de demostración
|   |-- lib.rs                  -> Raíz de la librería que expone los módulos
|   |-- cli.rs                  -> Parseo de argv y validación en Config (Demo, File, Deterministic, 
|   |                              Debug, Invalid)
|   |-- ui.rs                   -> Formateo de colores, headers y mensajes para impresión por terminal
|   |-- demos.rs                -> Definición de las 7 demostraciones hardcodeadas del intérprete
|   |-- runner.rs               -> Ejecución de los distintos modos: demos completas/particulares,    
|   |                              archivo determinístico/no determinístico, y modo debug (TUI)
|   |-- stats.rs                -> Estadística descriptiva y diagnósticos de convergencia (media, ESS, 
|   |                              autocorrelación)
|   |
|   |-- parser/                 -> Módulo de análisis sintáctico y AST
|   |   |-- mod.rs              -> Exportaciones del módulo de parsing
|   |   |-- sexpr.rs            -> Analizador de S-Expressions y generación del AST (sintaxis Lisp/
|   |   |                          Clojure)
|   |   |-- value.rs            -> Definición de RVal que uso como valor de retorno
|   |   |-- primitives.rs       -> Operaciones y funciones primitivas nativas
|   |   +-- distribution.rs     -> Abstracciones y matemática de distribuciones
|   |
|   |-- interpreter/            -> Motor de evaluación y tiempo de ejecución
|   |   |-- mod.rs              -> Exportaciones del evaluador
|   |   |-- machine.rs          -> Máquina de evaluación para entornos y Closures
|   |   +-- runtime.rs          -> Intérprete, direcciones (Addresses) e interfaz de mensajes para el 
|   |                              motor de inferencia
|   |
|   |-- inference/              -> Motores de inferencia probabilística
|   |   |-- mod.rs              -> Exportaciones de algoritmos
|   |   |-- defaults.rs         -> Constantes compartidas (N de partículas, steps, etc.) entre el modo 
|   |   |                          CLI y el modo debug
|   |   |-- bbvi.rs             -> Algoritmo: Black-Box Variational Inference (BBVI)
|   |   |-- exact_enumeration.rs -> Algoritmo: Exact Enumeration
|   |   |-- lw.rs               -> Algoritmo: Likelihood Weighting
|   |   |-- smc.rs              -> Algoritmo: Sequential Monte Carlo (SMC)
|   |   +-- ssmh.rs             -> Algoritmo: Single-Site Metropolis-Hastings
|   |
|   +-- debugger/               -> Debugger interactivo de terminal (TUI, ratatui)
|       |-- mod.rs              -> Exportaciones del módulo
|       |-- app.rs              -> DebuggerApp: loop principal, historial, breakpoints, log de eventos
|       |-- event.rs            -> Mapeo de teclas a comandos (step, continue, breakpoint, etc.)
|       |-- render.rs           -> Paneles ratatui (header, panel actual, log, ayuda)
|       +-- engine/             -> Un motor de "paso" por algoritmo de inferencia
|           |-- mod.rs          -> Enum Engine que despacha al motor activo
|           |-- lw.rs           -> Motor de step para Likelihood Weighting
|           |-- enumeration.rs  -> Motor de step para Exact Enumeration
|           |-- smc.rs          -> Motor de step para Sequential Monte Carlo
|           |-- ssmh.rs         -> Motor de step para Single-Site Metropolis-Hastings
|           +-- bbvi.rs         -> Motor de step para Black-Box Variational Inference
|
+-- tests/                      -> Pruebas unitarias y de integración
    |-- parser_tests.rs         -> Pruebas de validación sintáctica y AST
    |-- interpreter_tests.rs    -> Pruebas de evaluación, recursión y Closures
    |-- distributions_tests.rs  -> Pruebas de densidad y distribuciones
    |-- primitives_tests.rs     -> Pruebas para operaciones primitivas incluido distribuciones y 
    |                              operaciones sobre tipos de datos
    +-- inference_tests.rs      -> Pruebas de convergencia de los algoritmos
```

## 🦀 Lenguaje de Implementación: ¿Por qué Rust?

Aunque la consigna permitía usar lenguajes interpretados de alto nivel como Python, decidí escribir todo el proyecto (el *lexer*, el *parser* de **HOPPL** y el motor de evaluación) desde cero en **Rust**. La decisión se apoya en cuatro razones concretas:

### 1. Pattern Matching y tipos algebraicos (ADTs) para el AST
Un intérprete pasa la mayor parte del tiempo manipulando árboles de sintaxis abstracta (AST) y evaluando expresiones recursivas.
* Los **enums** de Rust permiten modelar las expresiones del lenguaje (operaciones, closures, llamadas `sample`, `observe`, etc.) de forma directa.
* El **Pattern Matching exhaustivo** (`match`) obliga al evaluador a cubrir todos los casos del AST. Si se agrega un nodo o una primitiva nueva, el compilador señala exactamente qué partes del evaluador quedaron sin actualizar.

### 2. Seguridad de tipos y errores detectados en tiempo de compilación
A diferencia de lenguajes con tipado dinámico, donde los errores de lógica o de memoria aparecen recién en tiempo de ejecución (a veces a mitad de una simulación larga), el sistema de tipos y el *Borrow Checker* de Rust atajan buena parte de esos problemas antes de correr el programa:
* **Sin referencias nulas:** usar `Option<T>` y `Result<T, E>` obliga a manejar de forma explícita los errores sintácticos del *Parser* y los semánticos del *Evaluador*.
* **Seguridad de memoria sin Garbage Collector:** se evitan fugas de memoria y errores de segmentación sin pagar el costo de las pausas de un recolector de basura.

### 3. Rendimiento en los algoritmos de inferencia
Los algoritmos implementados (*Sequential Monte Carlo*, *Metropolis-Hastings*, *Likelihood Weighting*) hacen bastante trabajo computacional:
* En **SMC**, por ejemplo, hay que mantener, evaluar y clonar miles de "partículas" (trazas de ejecución) en paralelo y hacer re-muestreos (*resampling*) todo el tiempo.
* Al compilar a código de máquina nativo con abstracciones de costo cero (*zero-cost abstractions*), Rust corre miles de pasos de MCMC o partículas de SMC en una fracción del tiempo que tomaría en Python, con un rendimiento cercano al de C o C++.

### 4. Manejo de la traza (Trace Tracking)
Implementar algoritmos como **Single-Site Metropolis-Hastings** requiere mantener una "traza" (*Trace*) que asocie cada llamada `sample` con su dirección de ejecución (*Address*). El sistema de *Ownership* de Rust, junto con el clonado explícito de datos, hace más simple llevar ese seguimiento de direcciones sin efectos secundarios inesperados al modificar el estado de las variables aleatorias.

## Aclaraciones Técnicas: CPS Funcional Puro vs. Máquina CEK

Durante el diseño del proyecto, y a partir de una sugerencia del profesor, evalué implementar el evaluador de expresiones con **Continuation-Passing Style (CPS) funcional puro**. En la literatura clásica de Lisp esto se logra pasando funciones de orden superior (*closures*) como continuaciones para pausar y reanudar el flujo.

Terminé descartando el CPS puro y usando en su lugar una **Máquina CEK (Control, Environment, Continuation)**, que es la "defuncionalización" matemática del CPS. La decisión resuelve dos problemas concretos que aparecen al hacer esto en Rust:

1. **La barrera de la clonación (función `fork` para SMC y MCMC):** este fue el factor decisivo. Algoritmos como Sequential Monte Carlo necesitan pausar la ejecución en cada `observe`, **clonar** el estado de la máquina en múltiples partículas y reanudar en paralelo. En Rust es complejo clonar un *closure* arbitrario escondido detrás de *Traits* dinámicos (por ejemplo `Box<dyn Fn>`), porque el compilador no conoce el tamaño ni el contenido del entorno capturado en tiempo de ejecución. Con una Máquina CEK, la "continuación" deja de ser una función opaca y pasa a ser una estructura de datos concreta (un vector de enums `Vec<Instr>`), que se puede clonar con un simple `#[derive(Clone)]`.

2. **Tipos opacos y TCO (Tail Call Optimization):** implementar CPS puro implica construir tipos de retorno recursivos y encadenar *closures*. En Rust, lidiar con los *lifetimes* de referencias dentro de closures anidados complica bastante la legibilidad y el mantenimiento del evaluador. Además, como Rust no garantiza optimización de llamadas de cola (TCO), un CPS puro con recursión probabilística profunda terminaría en un *Stack Overflow*. La pila explícita de la máquina CEK maneja el flujo de forma iterativa en el *Heap*, evitando ese problema.

# Extras

En esta sección vamos a hablar de cosas agregadas por fuera de la consigna para hacer un trabajo práctico más completo.

## Diagnósticos de Convergencia MCMC y Métricas (`src/stats.rs`)

Para evaluar rigurosamente la calidad de las cadenas generadas por los algoritmos de inferencia y garantizar la validez estadística de los resultados aproximados, el módulo `stats.rs` calcula y reporta tres métricas de diagnóstico clave al finalizar la ejecución:

### 1. Intervalo de Confianza del 95% (95% CI)
Indica los percentiles $2.5\%$ y $97.5\%$ de la distribución empírica marginal obtenida de las muestras. Provee una región de alta densidad de probabilidad que permite localizar dónde se concentra el valor real de los parámetros latentes con un nivel de significancia estadística estándar.

### 2. Tamaño de Muestra Efectivo Porcentual (ESS%)
Debido a la naturaleza secuencial y estocástica de algoritmos como Metropolis-Hastings, las muestras sucesivas de la cadena suelen estar fuertemente autocorrelacionadas. El **Tamaño de Muestra Efectivo (ESS)** estima cuántas muestras independientes y *no correlacionadas* contiene la traza real:

$$\text{ESS} = \frac{N}{1 + 2 \sum_{k=1}^{\infty} \rho_k}$$

Donde $N$ es el tamaño total de la muestra y $\rho_k$ es la autocorrelación al lag $k$.
* **ESS%:** Representa la relación porcentual $(\text{ESS} / N) \times 100$. Un ESS% bajo (ej. $< 5\%$) alerta al desarrollador sobre una fuerte correlación y una mezcla deficiente (*poor mixing*), sugiriendo la necesidad de incrementar el tamaño de la cadena o ajustar las propuestas.

### 3. Tasa de Aceptación (Acceptance Rate)
Métrica específica de los algoritmos MCMC (como Single-Site MH) que mide la proporción de estados propuestos que fueron aceptados sobre el total de pasos iterados:

$$\text{Tasa de Aceptación} = \frac{\text{Propuestas Aceptadas}}{\text{Total de Iteraciones}}$$

* **Interpretación:** Permite ajustar el tamaño de los pasos de las distribuciones de propuesta. Una tasa excesivamente alta indica que el algoritmo está dando pasos muy pequeños explorando de forma ineficiente, mientras que una tasa muy baja refleja que la mayoría de los saltos son rechazados, estancando la cadena en el mismo estado.

## Parser (`src/parser/sexpr.rs`)

Para este proyecto se implementó un flujo completo de análisis sintáctico desde cero en **Rust** para procesar **S-Expressions** (expresiones S), el formato de sintaxis clásico de la familia Lisp/Clojure. El analizador sintáctico realiza la traducción directa de código en formato de texto plano a un Árbol de Sintaxis Abstracta (AST) tipado y seguro.

A diferencia del tipado dinámico implícito de la implementación original en Python, la versión en Rust modela todo el sistema usando **Tipos de Datos Algebraicos (ADTs)** mediante Enums y Pattern Matching.

El proceso de parseo está dividido en dos etapas principales:

### 1. El Tokenizador o Lexer (`tokenize`)
El tokenizador escanea secuencialmente el flujo de caracteres del código fuente y lo agrupa en un vector de tokens internos definidos por el enum `Token`:
* `Token::LParen`: Representa un delimitador de apertura. Se unifican tanto los paréntesis `(` como los corchetes `[` bajo este token.
* `Token::RParen`: Representa un delimitador de cierre, unificando tanto `)` como `]`.
* `Token::StringLit(String)`: Cadenas de texto literales (por ejemplo, `"resultado"`).
* `Token::Atom(String)`: Identificadores, números y símbolos (por ejemplo, `+`, `x`, `42`, `3.14`).

**Características del Lexer:**
* **Espacios y Comentarios:** Se ignoran espacios en blanco, tabulaciones, saltos de línea y comas `,` (que actúan como separadores de legibilidad en Clojure). Los comentarios que inician con punto y coma (`;`) se omiten por completo hasta el final de la línea.
* **Soporte de Strings y Escapes:** Las cadenas encerradas en comillas dobles (`"..."`) soportan secuencias de escape estándar (`\n`, `\t`, `\\`, `\"`). Si una cadena queda abierta, el lexer lanza un error sintáctico preciso en lugar de fallar de forma silenciosa.

### 2. El Parser Recursivo Descendente (`read_form`)
El analizador sintáctico consume el vector de tokens de manera recursiva y construye el AST, el cual está representado mediante la estructura `Form`:
```rust
pub enum Form {
    Symbol(String), // Identificadores (variables, primitivas, etc.)
    Int(i64),       // Enteros de 64 bits
    Float(f64),     // Flotantes de 64 bits
    Bool(bool),     // Booleanos (true/false)
    Str(String),    // Cadenas de texto literales
    Nil,            // Valor nulo (nil)
    List(Vec<Form>),// Expresiones compuestas/anidadas
}
```

**Mecanismos Clave del Parser:**
* **Conversión de Átomos (`atom`):** Convierte tokens de texto en variantes específicas de `Form`. Primero busca palabras clave como `true`, `false` y `nil`. Si no coinciden, intenta parsearlos como enteros de 64 bits o flotantes de 64 bits de forma estricta. Si falla el parseo numérico, se catalogan de forma segura como `Form::Symbol`.
* **Análisis de Listas Recursivo:** Cuando detecta un `Token::LParen`, abre una nueva lista y procesa de forma recursiva todos los sub-elementos hasta encontrar su correspondiente `Token::RParen`.
* **Manejo Robusto de Errores Sintácticos:** En lugar de entrar en pánicos o retornar resultados inconsistentes, el parser detecta desbalances y reporta errores descriptivos con indicaciones claras del problema (por ejemplo, paréntesis o corchetes abiertos que nunca se cerraron).

### API Pública del Módulo
El módulo expone una interfaz limpia para ser consumida por el evaluador o los motores de inferencia:
* `parse(text: &str) -> Result<Vec<Form>, String>`: Procesa un programa completo y devuelve una lista de todas las formas (expresiones) de nivel superior encontradas.
* `parse_one(text: &str) -> Result<Form, String>`: Utilizado para procesar exactamente una única expresión. Lanza un error amigable si el texto está vacío o si contiene múltiples formas de nivel superior sueltas.
* `to_string(form: &Form) -> String`: Función inversa que toma un nodo del AST y lo renderiza de vuelta como código legible de Clojure (preservando el formato `.0` para floats sin parte decimal, asegurando la consistencia de tipos).

## Detección Estática de Desincronización de SMC

En el algoritmo de inferencia **Sequential Monte Carlo (SMC)** (o Filtro de Partículas), todas las partículas representan trazas de ejecución concurrentes que deben avanzar de forma sincronizada. Específicamente, cada vez que las partículas se topan con una instrucción `observe`, deben detenerse al unísono (punto de sincronización) para evaluar la verosimilitud del valor observado, actualizar sus pesos acumulados y participar en el proceso coordinado de re-muestreo multinomial (*resampling*).

Si alguna partícula tomara un camino alternativo donde no ejecuta un `observe` que las demás sí ejecutan (o viceversa), la traza se desincroniza y rompe la consistencia matemática del algoritmo.

Para evitar este riesgo, el proyecto implementa **dos capas de defensa**: un **análisis estático** antes de la ejecución y una **detección dinámica** en tiempo de ejecución.

### 1. El Análisis Estático Previo a la Ejecución

Al inicio de la función principal `run_smc`, antes de inicializar la máquina de evaluación o crear las partículas, se parsea el programa y se realiza una inspección exhaustiva de su estructura del Árbol de Sintaxis Abstracta (AST):

```rust
pub fn run_smc<R: Rng + ?Sized>(...) -> Result<Vec<RVal>, String> {
    // 1. Parseamos el código fuente a su representación AST (Form)
    let forms = parse(program)?;

    // 2. Realizamos la verificación estática de desincronización
    check_scm_safety(&forms)?;
    
    // ... resto del algoritmo SMC
}
```

La verificación se compone de dos funciones auxiliares principales:

* **`check_scm_safety(forms: &[Form]) -> Result<(), String>`**:
  Itera recursivamente sobre todas las expresiones de nivel superior (*top-level forms*) del código fuente llamando a `check_form`. Si alguna de ellas viola las reglas de seguridad estructural, interrumpe el arranque del algoritmo inmediatamente y propaga un mensaje de error descriptivo.

* **`check_form(form: &Form) -> Result<bool, String>`**:
  Es una función recursiva descendente que inspecciona el AST y tiene dos misiones:
  1. Retornar `Ok(true)` si la expresión actual o alguna de sus sub-expresiones contiene un `observe` (para notificar la presencia de observaciones hacia los nodos padres).
  2. Lanzar un `Err(String)` si localiza un `observe` en un contexto sintáctico que vulnere la sincronización determinística.

#### Patrones Prohibidos Detectados Estáticamente:
1. **`observe` dentro de ramas condicionales (`if`):**
   ```clojure
   (if condicion (observe (normal 0 1) 0.5) (sample (normal 0 1)))
   ```
   * **Por qué se prohíbe:** La condición del `if` puede depender del estado estocástico aleatorio de cada partícula. Si unas partículas evalúan la condición como `true` y otras como `false`, unas ejecutarán el `observe` y otras no, rompiendo inmediatamente la alineación de las trazas de SMC.
   * **Error reportado:** *"SMC Static Analysis Error: Found an 'observe' statement inside an 'if' branch. SMC requires a deterministic observation flow. Please move the observation outside the conditional."*

2. **`observe` dentro de definiciones de funciones (`fn` o `defn`):**
   ```clojure
   (let [mi-funcion (fn [x] (observe (normal x 1) 2.0))] ...)
   ```
   * **Por qué se prohíbe:** Las funciones y clausuras pueden almacenarse, pasarse como argumentos o invocarse dinámicamente un número arbitrario de veces (o ninguna) en tiempo de ejecución. Por ende, es matemáticamente imposible garantizar estáticamente la sincronización de las observaciones si estas residen dentro de una función.
   * **Error reportado:** *"SMC Static Analysis Error: Found an 'observe' statement inside a 'fn' definition. Functions can be called dynamically, which breaks SMC synchronization guarantees."*

3. **Propagación en Bloques `let`:**
   Rastrea la presencia de `observe` tanto en los valores asociados a variables como en las expresiones que conforman el cuerpo del bloque `let`, para que ninguna instrucción quede sin detectar.

---

### 2. La Salvaguarda Dinámica en Tiempo de Ejecución

Como red de seguridad complementaria, si existiera un flujo de ejecución dinámico sumamente complejo que lograra eludir el análisis estático y causara una desincronización real de las partículas en tiempo de ejecución, la función `run_smc` lo detecta inmediatamente.

Durante el bucle principal de avance, se avanzan todas las partículas en paralelo hasta que cada una de ellas se detiene en su próximo punto de sincronización (retornando una señal o mensaje `Msg`):

```rust
for msg in messages {
    match msg {
        Msg::Observe(_addr, dist, y_obs, mut m) => {
            // Flujo normal: todas las partículas están sincronizadas en un 'observe'
            ...
        }
        // Si alguna partícula terminó prematuramente (Done) o se detuvo por otra señal
        _ => return Err("SMC Desynchronization Error: Particles reached divergent execution states. All particles in Sequential Monte Carlo must encounter the exact same sequence of 'observe' statements.".into()),
    }
}
```

Con este esquema, cualquier desincronización se detecta antes o durante la ejecución, con un diagnóstico inmediato en vez de dejar correr una simulación que daría resultados incorrectos sin avisar.

## Algoritmos de Inferencia Extra

Como extra, el motor de inferencia también cubre 2 algoritmos adicionales vistos durante la cursada. La misma máquina virtual CEK sirve para los dos, sin cambios en su diseño:

### 1. Black-Box Variational Inference (BBVI)

A diferencia de los métodos tradicionales de Monte Carlo (MCMC/SMC) que aproximan la distribución a posteriori mediante muestreo estocástico, BBVI transforma la inferencia en un problema de **optimización matemática**.

* Se propone una familia de distribuciones guía parametrizadas $q_\theta(x)$ para cada sitio probabilístico.

* El objetivo es encontrar los parámetros $\theta$ que minimicen la divergencia con la distribución real, maximizando la cota inferior de la evidencia (**ELBO**).

* Para lograrlo sin requerir un motor de diferenciación automática global, el algoritmo utiliza el *Score Function Trick* (REINFORCE) y un **optimizador Adam** programado nativamente, el cual ajusta los parámetros mediante descenso/ascenso de gradiente estocástico.

**Nota:** Este algoritmo de inferencia se explora en el capítulo 4 del libro antes mencionado en el que se basa fuertemente la cursada.

### 2. Enumeración Exacta (Exact Enumeration)

Se trata de un método de inferencia **100% determinista y exacto**. En lugar de realizar estimaciones lanzando valores al azar, este algoritmo explora exhaustivamente todos los universos o ramificaciones posibles del programa.

* Cada vez que la ejecución alcanza una instrucción `sample`, la máquina virtual se "clona" (fork) a sí misma por cada posible valor que puede tomar la distribución, explorando todos los caminos en paralelo y calculando su probabilidad exacta mediante la regla de Bayes.

* **Limitación intrínseca:** La enumeración exacta requiere que las variables probabilísticas tengan **soporte finito** (solo distribuciones discretas acotadas, como Bernoulli o Categórica). Si se intenta enumerar una distribución continua (como la Normal, que posee infinitos resultados posibles), el motor arroja un error controlado para evitar una explosión combinatoria y el agotamiento de la memoria.


**Nota:** A diferencia de **BBVI** este algoritmo de inferencia no se menciona de manera explícita en el libro, pero fue visto en clase, esto es debido a su poca aplicabilidad en casos reales.

## Debugger Interactivo de Inferencia (TUI)

Se implementó un debugger de terminal (`src/debugger/`, construido con **ratatui** + **crossterm**) que permite ejecutar cualquier modelo HOPPL paso a paso en vez de correrlo de punta a punta:

```bash
cargo run -- debug <archivo.hoppl> <algoritmo>
```

La TUI se organiza en paneles (`src/debugger/render.rs`): un header con el estado, un panel **Model** que muestra el código fuente del programa que se está depurando (para tener siempre a la vista qué modelo se está ejecutando, sin tener que volver al archivo), el panel **Current** con el estado puntual del motor activo, un **Event log** con el historial de pasos, y un footer con los controles.

### El problema: no todos los algoritmos "pausan" de la misma forma

Un debugger ingenuo que simplemente pausa la máquina CEK en cada `sample`/`observe`/`factor` alcanza para **Likelihood Weighting** (es literalmente una única traza lineal), pero no describe la ejecución real de los otros cuatro algoritmos:

* **Exact Enumeration** no es una traza sino un árbol: en cada `sample` se bifurca una rama por cada valor del soporte finito.
* **SMC** avanza N partículas en lockstep, sincronizadas en cada `observe`, con resampling entre sincronizaciones: no hay "una" traza que pausar.
* **SSMH** no pausa en efectos: cada iteración re-corre el programa completo proponiendo un valor nuevo en una dirección y acepta/rechaza con Metropolis-Hastings.
* **BBVI** es optimización de gradiente por lotes de trazas: no hay direcciones que pausar, sólo iteraciones de entrenamiento.

Por eso, en vez de un único stepper genérico, `src/debugger/engine/` define un motor de "paso" por algoritmo (`lw.rs`, `enumeration.rs`, `smc.rs`, `ssmh.rs`, `bbvi.rs`), despachados mediante un `enum Engine` cerrado con `match` exhaustivo, la misma preferencia por **enums + pattern matching** sobre `Box<dyn Trait>` que ya se explicó en la sección de [CPS Funcional Puro vs. Máquina CEK](#aclaraciones-técnicas-cps-funcional-puro-vs-máquina-cek). Cada motor reutiliza la lógica real del algoritmo correspondiente en `src/inference/` (expuesta como `pub(crate)`) en lugar de reimplementarla, así el modo debug nunca diverge del modo de ejecución normal.

Qué significa "un paso" (`s`) en cada motor:

| Algoritmo | Un paso... |
|---|---|
| Likelihood Weighting | Avanza el próximo efecto (`sample`/`observe`/`factor`) de la traza. |
| Exact Enumeration | En cada `sample`, elegís con `↑`/`↓` qué valor del soporte finito explorar; las ramas hermanas quedan en una pila pendiente para explorarlas después. |
| SMC | Corre un round de sincronización completo: todas las partículas hasta el próximo `observe` (o el final), con resampling multinomial y ESS del round. |
| Single-Site MH | Corre una iteración completa de Metropolis-Hastings: propone un valor nuevo en una dirección al azar y muestra el `log_alpha` y el accept/reject. |
| BBVI | Corre un paso completo de optimización Adam sobre un lote de trazas, mostrando el ELBO y los parámetros variacionales `θ`. |

### Qué se muestra al terminar (`Done`)

Al llegar al final, cada motor reporta la misma estimación que reportaría el modo no interactivo (`cargo run -- <archivo> <algoritmo>`), calculada con las mismas funciones de `src/stats.rs` (`sample_mean_std_err`, `mcmc_mean_std_err_ess`, `ci95_margin`), reutilizadas a través de un helper común (`posterior_summary_lines` en `src/debugger/engine/mod.rs`) para que los cuatro paneles no diverjan entre sí ni del modo CLI:

| Algoritmo | Qué reporta al terminar |
|---|---|
| SMC | Media estimada ± error estándar y CI 95%, sobre las partículas finales. |
| Single-Site MH | Media, CI 95% y ESS ajustado por autocorrelación, sobre la cadena post-warmup. |
| BBVI | Media posterior (vía la guía optimizada) ± error estándar y CI 95%, sobre el último lote de trazas. |
| Exact Enumeration | Tabla completa P(valor) más la media ponderada por la PMF exacta (cuando el resultado es numérico), más informativa que una media sola ya que no es una aproximación. |
| Likelihood Weighting | Solo el resultado y el `log_w` de esa traza puntual. A diferencia de los otros cuatro, el motor de debug de LW pausa una única traza lineal en vez de correr las N partículas del modo no interactivo, así que no hay una media a posteriori que promediar. |

Si el resultado del modelo no es numérico (por ejemplo, devuelve un booleano), en vez de la media se muestra un desglose de frecuencias por valor.

### Controles

* `s`: avanzar un paso.
* `c`: continuar automáticamente hasta el próximo breakpoint o el final (o, en SSMH/BBVI, hasta el presupuesto de pasos configurado, ya que un chain de MCMC o un loop de optimización no terminan solos).
* `b`: alternar un breakpoint en la dirección actual (no aplica en BBVI, que no tiene direcciones donde pausar).
* `↑` / `↓`: elegir la rama resaltada (sólo tiene efecto en Exact Enumeration).
* `←` / `→`: navegar hacia atrás/adelante en el historial ya explorado (de solo lectura).
* `q` / `Esc`: salir.

## Futuras Características: Hacia una Plataforma de Experimentación Probabilística

Había identificado dos líneas de trabajo futuro para el proyecto, por fuera de lo pedido en la consigna. Las dos ya están implementadas:

> **Nota:** el operador de condicionamiento suave `factor` (originalmente listado acá como característica futura) ya fue implementado. Ver [Tutorial, sección 7](#7-condicionamiento-suave-con-factor) y la demo 7 (`cargo run -- 7`).
>
> **Nota:** el debugger de inferencia y visualización de trazas (originalmente el ítem 1 de esta sección) también ya fue implementado. Ver [Debugger Interactivo de Inferencia (TUI)](#debugger-interactivo-de-inferencia-tui) y el modo 5 de [Correr el proyecto](#5-depurar-un-modelo-hoppl-paso-a-paso-modo-debug).