# Trabajo Final Introducción a los Lenguajes de Programación Probabilísticos

Trabajo final de **Martín Nievas Wilberger** para la materia Introduccion a los Lenguajes de Programación Probabilísticos

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

# Estructura del Proyecto

El código fuente está organizado de forma modular siguiendo las convenciones e idiomáticas de **Rust**, separando claramente las etapas de análisis sintáctico (frontend), evaluación y ejecución (backend), los motores de inferencia matemática y la batería de pruebas:

```plaintext
TP-FINAL-PPL
|-- Cargo.lock
|-- Cargo.toml               -> Configuración y dependencias en Rust
|-- LICENSE                  -> Licencia del proyecto
|-- README.md                -> Documentación principal del proyecto
|-- src/
|   |-- main.rs              -> 
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
|   |   +-- runtime.rs       -> Interprete, direccion (Addresses) e interfaz de mensajes para el motoro de inferencia.
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

El entorno global provee un amplio conjunto de primitivas determinísticas para operar sobre los datos, modeladas a partir de Clojure y NumPy:

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

El lenguaje soporta la instanciación de variables aleatorias a través de diversas familias de distribuciones paramétricas. Todas las distribuciones implementan métodos internos para muestrear (`sample`) o evaluar log-densidades (`log_prob`).

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

# Ejecutar Proyecto

