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

## 🦀 Lenguaje de Implementación: ¿Por qué Rust?

Aunque la consigna del proyecto permitía utilizar lenguajes interpretados de alto nivel (como Python), se tomó la decisión arquitectónica de desarrollar el proyecto —incluyendo el *lexer*, el *parser* para la sintaxis del **HOPPL** (*Higher-Order Probabilistic Programming Language*) y el motor de evaluación— completamente desde cero en **Rust**. 

Esta elección se fundamenta en cuatro pilares críticos que aportan ventajas significativas al diseño de lenguajes y a la computación probabilística:

### 1. Pattern Matching y Tipos Algebraicos (ADTs) para el AST
El diseño de un intérprete requiere manipular continuamente árboles de sintaxis abstracta (AST) y evaluar expresiones recursivas. 
* Los **Enums potentes (tipos algebraicos)** de Rust permiten modelar las expresiones del lenguaje probabilístico (operaciones, closures, llamadas `sample`, `observe`, etc.) de forma natural y autoexplicativa.
* El **Pattern Matching exhaustivo** (`match`) garantiza que el evaluador maneje absolutamente todos los casos posibles y ramificaciones del AST. Si se agrega un nuevo nodo o primitiva al lenguaje, el compilador nos alertará exactamente de qué partes del evaluador necesitan ser actualizadas.

### 2. Seguridad de Tipos y Prevención de Errores en Tiempo de Compilación
A diferencia de lenguajes interpretados o con tipado dinámico donde los errores de lógica o de memoria explotan en tiempo de ejecución (a mitad de una simulación larga), el estricto sistema de tipos y el *Borrow Checker* de Rust actúan como una primera línea de defensa:
* **Cero excepciones en tiempo de ejecución por referencias nulas:** El uso de tipos monádicos como `Option<T>` y `Result<T, E>` hace que el manejo de errores sintácticos en el *Parser* y errores semánticos en el *Evaluador* sea explícito y predecible.
* **Seguridad de memoria sin Garbage Collector (GC):** Se evitan fugas de memoria y errores de segmentación sin pagar el costo de las pausas en tiempo de ejecución de un recolector de basura.

### 3. Rendimiento y Eficiencia en Algoritmos de Inferencia
Los algoritmos implementados en este proyecto (*Sequential Monte Carlo*, *Metropolis-Hastings* y *Likelihood Weighting*) son de naturaleza altamente intensiva a nivel computacional:
* Por ejemplo, en **Sequential Monte Carlo (SMC)** es necesario mantener, evaluar y clonar miles de "partículas" (trazas de ejecución de los grafos probabilísticos) en paralelo y realizar re-muestreos (*resampling*) constantes.
* Al ser un lenguaje compilado a código de máquina nativo con abstracciones de costo cero (*zero-cost abstractions*), Rust ejecuta simulaciones de miles de pasos de MCMC o partículas SMC en una fracción del tiempo que le tomaría a lenguajes interpretados como Python, logrando una eficiencia del orden de C o C++.

### 4. Ergonomía para la Estructura de Traza (Trace Tracking)
Para implementar algoritmos como **Single-Site Metropolis-Hastings**, es indispensable mantener una "traza" (*Trace*) que asocie de manera unívoca cada llamada condicional `sample` a una dirección de ejecución (*Address*). La propiedad de pertenencia (*Ownership*) y el clonado explícito de datos en Rust facilitan la creación de un sistema de seguimiento de direcciones limpio y sin efectos secundarios inesperados al modificar el estado de las variables aleatorias.

## Aclaraciones Técnicas: CPS Funcional Puro vs. Máquina CEK

Durante las iteraciones de diseño de este proyecto, y contemplando una sugerencia del profesor, se evaluó fuertemente la posibilidad de implementar el evaluador de expresiones utilizando **Continuation-Passing Style (CPS) funcional puro**. En la literatura clásica de Lisp, esto se logra pasando funciones de orden superior (*closures*) como continuaciones para pausar y reanudar el flujo. 

Sin embargo, se tomó la decisión arquitectónica final de prescindir del CPS puro y utilizar en su lugar una **Máquina CEK (Control, Environment, Continuation)**, la cual es esencialmente la "defuncionalización" matemática del CPS. Esta decisión resuelve dos problemas críticos que presenta Rust:

1. **La barrera de la clonación (Función `fork` para SMC y MCMC):** Este es el factor decisivo. Algoritmos como Sequential Monte Carlo requieren pausar la ejecución en cada instrucción `observe`, **clonar** el estado de la máquina en múltiples partículas, y reanudar de forma paralela. En Rust, es notoriamente complejo y limitante clonar un *closure* arbitrario oculto detrás de *Traits* dinámicos (ej. `Box<dyn Fn>`), ya que el compilador desconoce el tamaño y el contenido del entorno capturado en tiempo de ejecución. Al usar una Máquina CEK, la "continuación" pasa de ser una función opaca a una simple estructura de datos concreta (un vector de enums `Vec<Instr>`), haciendo que toda la máquina sea trivial y rápidamente clonable mediante `#[derive(Clone)]`.

2. **Tipos Opacos y TCO (Tail Call Optimization):** Implementar CPS puro requiere construir tipos de retorno recursivos y encadenar *closures*. En Rust, lidiar con los tiempos de vida (*lifetimes*) de referencias dentro de múltiples *closures* anidados entorpece enormemente la legibilidad y mantenibilidad del evaluador. Además, al carecer Rust de optimización de llamadas de cola (TCO) garantizada, un CPS funcional puro para programas con recursión probabilística profunda terminaría provocando irremediablemente un *Stack Overflow*. La pila explícita de la máquina CEK maneja iterativamente el flujo en el *Heap*, evadiendo este problema por completo.

# Estructura del Proyecto

El código fuente está organizado de forma modular siguiendo las convenciones e idiomáticas de **Rust**, separando claramente las etapas de análisis sintáctico (frontend), evaluación y ejecución (backend), los motores de inferencia matemática y la batería de pruebas:

```plaintext
TP-FINAL-PPL
|-- Cargo.lock
|-- Cargo.toml               -> Configuración y dependencias en Rust
|-- LICENSE                  -> Licencia del proyecto
|-- README.md                -> Documentación principal del proyecto
|-- src/
|   |-- main.rs              -> Punto de entrada y ejecutable de demostración
|   |-- lib.rs               -> Raíz de la librería que expone los módulos
|   |
|   |-- parser/              -> Módulo de análisis sintáctico y AST
|   |   |-- mod.rs           -> Exportaciones del módulo de parsing
|   |   |-- sexpr.rs         -> Analizador de S-Expressions y generación del AST (sintaxis Lisp/Clojure)
|   |   |-- value.rs         -> Definición de RVal que uso como valor de retorno
|   |   |-- primitives.rs    -> Operaciones y funciones primitivas nativas
|   |   +-- distribution.rs  -> Abstracciones y matemática de distribuciones
|   |
|   |-- interpreter/         -> Motor de evaluación y tiempo de ejecución
|   |   |-- mod.rs           -> Exportaciones del evaluador
|   |   |-- machine.rs       -> Máquina de evaluación para entornos y Closures
|   |   +-- runtime.rs       -> Intérprete, dirección (Addresses) e interfaz de mensajes para el motor de inferencia.
|   |
|   +-- inference/           -> Motores de inferencia probabilística
|       |-- mod.rs           -> Exportaciones de algoritmos
|       |-- lw.rs            -> Algoritmo: Likelihood Weighting
|       |-- smc.rs           -> Algoritmo: Sequential Monte Carlo (SMC)
|       +-- ssmh.rs          -> Algoritmo: Single-Site Metropolis-Hastings
|
+-- tests/                   -> Pruebas unitarias y de integración
    |-- parser_tests.rs      -> Pruebas de validación sintáctica y AST
    |-- interpreter_tests.rs -> Pruebas de evaluación, recursión y Closures
    |-- distributions_tests.rs -> Pruebas de densidad y distribuciones
    |-- primitives_tests.rs  -> Pruebas para operaciones primitivas incluido distribuciones y operaciones sobre tipos de datos
    +-- inference_tests.rs   -> Pruebas de convergencia de los algoritmos
```
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

# Extras

En esta sección vamos a hablar de cosas agregadas por fuera de la consigna para hacer un trabajo práctico más completo.

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

Si alguna partícula tomara un camino alternativo donde no ejecuta un `observe` que las demás sí ejecutan (o viceversa), se produciría una **desincronización catastrófica** de la traza, rompiendo la consistencia matemática del algoritmo.

Para mitigar este riesgo de forma absoluta, este proyecto implementa un sistema de **defensa en dos capas**: una capa preventiva de **análisis estático** antes de la ejecución y una salvaguarda de **detección dinámica** en tiempo de ejecución.

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
   Rastrea y propaga meticulosamente la presencia de `observe` tanto en los valores asociados a variables como en las expresiones que conforman el cuerpo del bloque `let`, asegurando que no se enmascare ninguna instrucción.

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

Gracias a este esquema híbrido, el motor de inferencia garantiza una ejecución del algoritmo SMC 100% matemáticamente rigurosa, proporcionando un diagnóstico inmediato del error al desarrollador y evitando simulaciones inútiles o silenciosamente incorrectas.

## Algoritmos de Inferencia Extra

Como extra en el motor de inferencia del proyecto también cubrí 2 algoritmos de inferencia adicionales vistos durante la cursada, lo que demuestra la versatilidad de la máquina virtual CEK para adaptarse a diferentes paradigmas estadísticos:

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

# Ejecutar Proyecto