# Final Project: Introduction to Probabilistic Programming Languages

Final project by **Martín Nievas Wilberger** for the course *Introduction to Probabilistic Programming Languages*.

---

## Table of Contents

- [About the Project](#about-the-project)
- [Running the Project](#running-the-project)
- [About the Language](#about-the-language)
- [Tutorial: Writing Your First HOPPL Program](#tutorial-writing-your-first-hoppl-program)
- [Project Structure](#project-structure)
- [Implementation Language: Why Rust?](#implementation-language-why-rust)
- [Technical Notes: Pure Functional CPS vs. CEK Machine](#technical-notes-pure-functional-cps-vs-cek-machine)
- [Extras](#extras)

---

## About the Project

The main goal of this work is the design and implementation, from scratch, of a **Higher-Order Probabilistic Programming Language (HOPPL)**. The formal and semantic specification follows the framework presented in the reference book and paper *"An Introduction to Probabilistic Programming"* by **Jan-Willem van de Meent, Brooks Paige, Hongseok Yang and Frank Wood**.

### Requirements and Scope

The project fulfills the two core requirements of the assignment:

1. **Language capabilities.** The language is not limited to static first-order models. It natively supports advanced constructs that enable dynamic, variable-length computation graphs:
   - **Closures:** first-class functions that capture their lexical environment.
   - **Recursion:** recursive function definitions, essential for modeling complex stochastic processes (such as geometric distributions or probabilistic trees).

2. **Inference engines.** Implementation of the three most fundamental and widely used inference algorithms in the probabilistic programming paradigm for approximating posterior distributions:
   - **Likelihood Weighting**
   - **Sequential Monte Carlo (SMC)** (particle filter)
   - **Single-Site Metropolis-Hastings (MH)**

> **Note on technology:** the assignment allowed a free choice of implementation language for the interpreter. The technical details, architectural justifications and benefits of the chosen technology are covered in [Implementation Language: Why Rust?](#implementation-language-why-rust).

---

## Running the Project

This section lists all the project's dependencies and explains how to run it.

### Dependencies

As mentioned above, this is where Rust shines, and more specifically its build system and package manager, **Cargo**. It is installed as follows.

#### Installing Rustup

**Rustup** is the official Rust installer and toolchain manager. It installs `rustc` (the compiler) and `cargo` (the build system and package manager) and keeps them up to date.

##### macOS / Linux

Open a terminal and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This downloads the official rustup script and runs it. It will ask which kind of installation you want (option `1) Proceed with installation (default)` is enough for most cases).

Once finished, load the environment variables into the current session (or just open a new terminal):

```bash
source "$HOME/.cargo/env"
```

On macOS, if the Xcode command line tools (needed for compiling) are not installed, rustup will ask you to install them. You can do it ahead of time with:

```bash
xcode-select --install
```

##### Windows

There are two options:

1. **Graphical installer (recommended):** download and run [`rustup-init.exe`](https://rustup.rs) from the official site. The installer automatically detects whether the **Visual Studio C++ Build Tools** (required for linking on Windows) are missing and offers to install them.

2. **With winget (PowerShell):**

   ```powershell
   winget install Rustlang.Rustup
   ```

In both cases, **restart the terminal** after installing so the environment variables (`PATH`) are updated correctly.

##### Verifying the installation

On any of the three systems, verify with:

```bash
rustc --version
cargo --version
```

You should see the installed versions (for example `rustc 1.8x.x` and `cargo 1.8x.x`). If you get "command not found", you probably need to restart the terminal or reload your `PATH`.

### Building the Project

Once the repository is cloned, go to the project root (where `Cargo.toml` lives) and run:

```bash
cargo build --release
```

This automatically downloads all the dependencies (crates) declared in `Cargo.toml` and compiles the project in optimized mode. The first build may take a few minutes because it fetches and builds every dependency; subsequent builds are incremental and much faster.

> If you only want a quick development build (no optimizations), plain `cargo build` works too.

### Usage

The binary supports five different modes.

#### 1. Run all hardcoded demos

```bash
cargo run
```

Runs, in order, the 7 demos included in the project (Likelihood Weighting, SMC, SMC static safety, Single-Site MH, BBVI, Exact Enumeration, and `factor` tests), pausing between each one so you can read the results before continuing.

#### 2. Run a specific demo

```bash
cargo run -- <number>
```

Where `<number>` is a value between `1` and `7`. For example:

```bash
cargo run -- 4
```

runs only the Single-Site Metropolis-Hastings demo, with no pauses and without the other demos.

#### 3. Run your own `.hoppl` model

```bash
cargo run -- <file.hoppl> <algorithm>
```

Where:

- `<file.hoppl>` is the path to a text file containing a program written in HOPPL.
- `<algorithm>` is the inference engine to use. Supported values:

| Algorithm | Value to pass |
|---|---|
| Likelihood Weighting | `lw` |
| Single-Site Metropolis-Hastings | `ssmh` |
| Sequential Monte Carlo | `smc` |
| Black-Box Variational Inference | `bbvi` |
| Exact Enumeration | `exact-enumeration` (aliases: `enum`, `exact`) |

For example:

```bash
cargo run -- modelos/mi_modelo.hoppl smc
```

#### 4. Run your own deterministic `.hoppl` program

```bash
cargo run -- <file.hoppl>
```

Where `<file.hoppl>` is the path to a text file containing a deterministic program written in HOPPL.

#### 5. Debug a `.hoppl` model step by step (debug mode)

```bash
cargo run -- debug <file.hoppl> <algorithm>
```

Launches a terminal user interface (TUI, built with `ratatui`) that lets you execute the program step by step instead of running it end to end. The `<algorithm>` argument is not cosmetic: each of the 5 inference engines has its own notion of a "step", faithful to the real algorithm (see [Interactive Inference Debugger](#interactive-inference-debugger-tui) under Extras). For example:

```bash
cargo run -- debug modelos/mi_modelo.hoppl ssmh
```

`programs/` includes models designed specifically for trying out debug mode with each algorithm:

| Model | Intended for | Notes |
|---|---|---|
| `coin_bias.hoppl` | SMC / LW | several synchronized `observe` statements |
| `line_fit.hoppl` | SSMH / BBVI | two continuous latent variables |
| `three_coins.hoppl` | Exact Enumeration | 8-branch tree |
| `multi_factor.hoppl` | any | two consecutive `factor` statements |

Each one includes a comment with the exact command to run it.

### Running the Tests

The project includes automated tests that validate the parser, the interpreter, the inference algorithms, and the supported primitives and distributions. To run the **entire** test suite:

```bash
cargo test
```

To run only the tests of a specific module or file (for example, only the parser's or a particular inference algorithm's), pass a name filter:

```bash
cargo test <test_or_module_name>
```

Cargo runs only the tests whose name (or containing module path) matches the given filter. For example:

```bash
cargo test tests_parser
```

would run only the tests related to AST construction and tokenization.

> **Tip:** add the `-- --nocapture` flag if you want to see the `println!` output of the tests while they run (by default Cargo hides the standard output of passing tests):
>
> ```bash
> cargo test -- --nocapture
> ```

---

## About the Language

This language, strongly inspired by Lisp and Clojure, uses a syntax based on **S-expressions**, where code and data share the same nested-list structure. All code is evaluated by the CEK virtual machine, which interprets these lists to control flow, manage variables and trigger probabilistic effects.

### Syntax

#### Primitive Data Types

- **Symbols (identifiers):** character sequences used for variables and functions. E.g. `x`, `+`, `my-variable`.
- **Numbers:**
  - 64-bit integers. E.g. `42`, `-10`.
  - 64-bit floats. E.g. `3.14`, `-0.5`.
- **Booleans:** `true` and `false`.
- **Strings:** enclosed in double quotes, with support for escape characters. E.g. `"Hello World\n"`.
- **Nil:** represents the absence of a value. Written as `nil`.

#### Structures and Special Forms

- **Lists:** group expressions. The first element of a list is evaluated as the function or operator, and the rest as its arguments. E.g. `(+ 1 2)`.
- **Brackets:** syntactically identical to parentheses; by convention they are used to improve readability in parameter and variable definitions. E.g. `[x 1 y 2]` is equivalent to `(x 1 y 2)`.
- **Comments:** start with a semicolon (`;`) and extend to the end of the line.
- **Control flow and variables:**
  - `(let [var1 expr1 var2 expr2] body)`: binds local variables sequentially.
  - `(if condition then-branch else-branch)`: conditional branching.
  - `(fn [arg1 arg2] body)`: defines anonymous functions (closures).
- **Probabilistic effects:**
  - `(sample distribution)`: draws a random value from a distribution. Returns control to the inference engine.
  - `(observe distribution value)`: conditions the probabilistic model by observing that a distribution generated a specific value, adjusting the trace weights.

### Supported Operations

The global environment provides a broad set of deterministic primitives for operating on data, functions for manipulating data structures, and distributions for `sample` and `observe` statements. All deterministic operations are defined in `src/parser/primitives.rs` in a HashMap whose key is the symbol and whose value is the corresponding Rust function (distributions are also included there).

#### Arithmetic and Math

- Basic: `+`, `-`, `*`, `/`, `mod`
- Advanced: `sqrt`, `exp`, `log`, `pow`, `abs`, `floor`, `ceil`, `tanh`, `max`, `min`

#### Logic and Comparison

- Equality: `=`, `==`, `!=`
- Relational: `<`, `>`, `<=`, `>=`
- Boolean: `and`, `or`, `not`

#### Data Structures (Lists and Maps)

- Creation: `vector` (or `list`), `hash-map`, `range`
- Access: `get`, `first`, `second`, `last`, `nth`, `peek`
- Modification: `put` (or `assoc`), `rest`, `conj`, `cons`, `append`, `concat`
- Utility: `count`, `empty?`
- Type predicates: `vector?`, `map?`, `number?`

#### Matrix Operations

Native support for two-dimensional linear algebra (fundamental for Machine Learning models and Bayesian Neural Networks):

- `mat-mul`: matrix multiplication (dot product).
- `mat-add`: matrix addition.
- `mat-transpose`: matrix transposition.
- `mat-tanh`, `mat-relu`: activation functions applied element-wise.
- `mat-repmat`: matrix repetition (tiling), equivalent to `np.tile`.

### Supported Distributions

The language supports instantiating random variables from several parametric distribution families. All distributions implement internal methods for sampling (`sample`) and evaluating log-densities (`log_prob`). They are defined in the `src/parser/distribution.rs` module.

#### Continuous Distributions

- `(normal mu sigma)`: Normal (Gaussian) distribution.
- `(log-normal mu sigma)`: Log-Normal distribution.
- `(uniform a b)` / `(uniform-continuous a b)`: continuous Uniform distribution on the interval $[a, b]$.
- `(exponential rate)`: Exponential distribution.
- `(beta alpha beta)`: Beta distribution.
- `(gamma shape rate)`: Gamma distribution.
- `(dirichlet [alphas...])`: Dirichlet distribution (multivariate).

#### Discrete Distributions

- `(bernoulli p)` / `(flip p)`: Bernoulli trial (biased coin).
- `(poisson lam)`: Poisson distribution.
- `(discrete [probs...])` / `(categorical [probs...])`: Categorical distribution given a list of probabilities (automatically normalized).
- `(uniform-discrete lo hi)`: discrete Uniform distribution on the interval $[lo, hi)$.

---

## Tutorial: Writing Your First HOPPL Program

This section is a practical, progressive guide to learning how to write HOPPL programs, going from simple expressions to a complete probabilistic model. All examples can be tried by pasting them into a `.hoppl` file and running `cargo run -- file.hoppl <algorithm>` (see [Running the Project](#running-the-project)).

### 1. Expressions and Arithmetic

As in any Lisp, the first element of a list is the operator and the rest are its arguments:

```clojure
(+ 1 2)              ; -> 3
(* 3 (+ 1 1))        ; -> 6
(> 5 3)              ; -> true
```

### 2. Variables with `let`

`let` binds one or more local variables, in order, and evaluates a final body that uses them:

```clojure
(let [x 5
      y (+ x 2)]
  (* x y))           ; -> 35
```

Note that `y` can use `x` because `let` binds its variables sequentially (like Lisp's `let*`), not simultaneously.

### 3. Conditionals with `if`

```clojure
(let [x 10]
  (if (> x 5)
      "big"
      "small"))      ; -> "big"
```

### 4. Functions with `fn`

`fn` defines an anonymous function (closure). You can assign it to a name with `let` to reuse it:

```clojure
(let [square (fn [x] (* x x))]
  (square 4))        ; -> 16
```

The language also supports recursion, but with an important caveat: `let` is not a `letrec`. When `(fn [...] body)` is evaluated, the closure captures the environment as it is at that instant, which is *before* `let` finishes binding the function's name. Therefore, a function cannot call itself simply by its own name inside a `let`.

The standard way to achieve recursion here is the classic **self-application** trick: the function receives a copy of itself as an explicit argument and passes it along again on every recursive call. With this we can write, for example, a recursively implemented geometric distribution:

```clojure
; Counts how many "failures" (bernoulli p = false) occur before the first "success".
; This is exactly the definition of a Geometric(p) distribution.
(let [geometric
        (fn [self]
          (fn [p]
            (if (sample (bernoulli p))
                0
                (+ 1 ((self self) p)))))]
  ((geometric geometric) 0.3))
```

`(geometric geometric)` is applied to itself to produce the actual one-argument function (`fn [p] ...`), already with `self` correctly bound in its environment because, unlike the name in `let`, `self` is a function parameter and is resolved normally at call time. Inside the body, `((self self) p)` repeats the same trick for the recursive call.

This also shows something important about the language: since `sample` can return a different value each time it is evaluated, the number of times `geometric` calls itself varies from run to run. It is a simple example of a **variable-length** computation graph, one of the central requirements of a HOPPL.

### 5. Probabilistic Effects: `sample` and `observe`

- `(sample dist)` draws a random value from a distribution.
- `(observe dist value)` tells the inference engine "assume `dist` generated exactly `value`", conditioning the model.

```clojure
; mu is a latent random variable with a Normal(0, 1) prior
(let [mu (sample (normal 0 1))]
  ; We observe that, under Normal(mu, 1), the generated value was 3.0
  (observe (normal mu 1) 3.0)
  mu)
```

This program by itself does not "do" anything deterministic: it defines a probabilistic model. It needs an inference engine (`lw`, `ssmh`, `smc`, `bbvi` or `exact-enumeration`) to approximate the posterior distribution of `mu` given that we observed 3.0.

### 6. A Complete Model: Biased Coin (Beta-Bernoulli)

Putting everything together, here is a classic complete Bayesian model: we want to infer the bias `p` of a coin after observing 3 heads and 1 tail.

```clojure
; Prior: p ~ Beta(2, 2)
; Likelihood: we observe 3 heads (true) and 1 tail (false)
(let [p (sample (beta 2.0 2.0))]
    (observe (bernoulli p) true)
    (observe (bernoulli p) true)
    (observe (bernoulli p) true)
    (observe (bernoulli p) false)
    p)
```

Save this as `coin.hoppl` and run, for example:

```bash
cargo run -- coin.hoppl smc
```

to approximate the posterior distribution of `p` using Sequential Monte Carlo. You can replace `smc` with `lw`, `ssmh` or `bbvi` to compare how each inference engine handles the same model.

### 7. Soft Conditioning with `factor`

`observe` is actually a particular case of a more general operation: adding log-likelihood density to the execution trace. `observe` does it indirectly: you pass a distribution and a value, and the engine computes `log_prob(value)` for you. The `(factor <expr>)` operator gives you direct access to that mechanism: it adds the number you pass, as is, to the trace's accumulated log-weight, with no need for a distribution or a concrete observed value.

```clojure
(factor <expr>)
```

This is useful when what you want to model is not "I observed exactly this value" but a more flexible notion of "this configuration is more or less plausible". For example, you can hand-write the Gaussian density that `observe` would use internally:

```clojure
; Equivalent (up to the normalization constant) to:
;   (observe (normal mu 1.0) 3.0)
(let [mu (sample (normal 0.0 10.0))
      diff (- mu 3.0)
      log_lik (* -0.5 (* diff diff))]
    (factor log_lik)
    mu)
```

The key difference from `observe` is that `factor` does not compare against an exact data point: it forces you to write the density function yourself (or any other "how good is this state" function), instead of delegating it to a named distribution. This enables models where the evidence is not a fixed point but a continuous preference, for example penalizing configurations far from a desired value without fixing that value as a point observation:

```clojure
; We prefer p to be close to 0.5, without observing any concrete data.
(let [p (sample (beta 2.0 2.0))
      penalty (* -2.0 (* (- p 0.5) (- p 0.5)))]
    (factor penalty)
    p)
```

> **Important:** unlike `sample`, `factor` does not hand control back to the inference engine: there is no stochastic decision to make, just a number to add. That is why you can use it with any of the supported inference algorithms (`lw`, `ssmh`, `smc`) without the machine pausing at that point. As the expression's return value, `(factor <expr>)` always produces `nil`, so it is generally used as an intermediate statement inside a `let`, not as the final value of a body.

### 8. Next Steps

From here on, the sections [About the Language](#about-the-language) (above) and [Extras](#extras) (below) document all the primitives, distributions and static safety guarantees (for example, which patterns the SMC analyzer rejects) that you will need to write more complex models.

---

## Project Structure

The source code is organized in a modular fashion following **Rust** conventions and idioms, clearly separating the parsing stage (frontend), evaluation and execution (backend), the mathematical inference engines, and the test suite:

```plaintext
TP-FINAL-PPL
|-- Cargo.lock
|-- Cargo.toml                  -> Rust configuration and dependencies
|-- LICENSE                     -> Project license
|-- README.md                   -> Main project documentation
|-- programs/                   -> Directory of HOPPL programs
|-- src/
|   |-- main.rs                 -> Entry point and demo executable
|   |-- lib.rs                  -> Library root exposing the modules
|   |-- cli.rs                  -> argv parsing and validation into Config (Demo, File, Deterministic,
|   |                              Debug, Invalid)
|   |-- ui.rs                   -> Colors, headers and message formatting for terminal output
|   |-- demos.rs                -> Definition of the 7 hardcoded interpreter demos
|   |-- runner.rs               -> Execution of the different modes: full/specific demos,
|   |                              deterministic/non-deterministic file, and debug mode (TUI)
|   |-- stats.rs                -> Descriptive statistics and convergence diagnostics (mean, ESS,
|   |                              autocorrelation)
|   |
|   |-- parser/                 -> Parsing module and AST
|   |   |-- mod.rs              -> Parsing module exports
|   |   |-- sexpr.rs            -> S-expression parser and AST generation (Lisp/Clojure syntax)
|   |   |-- value.rs            -> Definition of RVal, used as the return value
|   |   |-- primitives.rs       -> Native primitive operations and functions
|   |   +-- distribution.rs     -> Distribution abstractions and math
|   |
|   |-- interpreter/            -> Evaluation engine and runtime
|   |   |-- mod.rs              -> Evaluator exports
|   |   |-- machine.rs          -> Evaluation machine for environments and closures
|   |   +-- runtime.rs          -> Interpreter, addresses and message interface for the
|   |                              inference engine
|   |
|   |-- inference/              -> Probabilistic inference engines
|   |   |-- mod.rs              -> Algorithm exports
|   |   |-- defaults.rs         -> Constants shared between CLI mode and debug mode (number of
|   |   |                          particles, steps, etc.)
|   |   |-- bbvi.rs             -> Algorithm: Black-Box Variational Inference (BBVI)
|   |   |-- exact_enumeration.rs -> Algorithm: Exact Enumeration
|   |   |-- lw.rs               -> Algorithm: Likelihood Weighting
|   |   |-- smc.rs              -> Algorithm: Sequential Monte Carlo (SMC)
|   |   +-- ssmh.rs             -> Algorithm: Single-Site Metropolis-Hastings
|   |
|   +-- debugger/               -> Interactive terminal debugger (TUI, ratatui)
|       |-- mod.rs              -> Module exports
|       |-- app.rs              -> DebuggerApp: main loop, history, breakpoints, event log
|       |-- event.rs            -> Key-to-command mapping (step, continue, breakpoint, etc.)
|       |-- render.rs           -> ratatui panels (header, current panel, log, help)
|       +-- engine/             -> One "step" engine per inference algorithm
|           |-- mod.rs          -> Engine enum dispatching to the active engine
|           |-- lw.rs           -> Step engine for Likelihood Weighting
|           |-- enumeration.rs  -> Step engine for Exact Enumeration
|           |-- smc.rs          -> Step engine for Sequential Monte Carlo
|           |-- ssmh.rs         -> Step engine for Single-Site Metropolis-Hastings
|           +-- bbvi.rs         -> Step engine for Black-Box Variational Inference
|
+-- tests/                      -> Unit and integration tests
    |-- parser_tests.rs         -> Syntax validation and AST tests
    |-- interpreter_tests.rs    -> Evaluation, recursion and closure tests
    |-- distributions_tests.rs  -> Density and distribution tests
    |-- primitives_tests.rs     -> Tests for primitive operations, including distributions and
    |                              operations on data types
    +-- inference_tests.rs      -> Algorithm convergence tests
```

---

## Implementation Language: Why Rust?

Although the assignment allowed interpreted high-level languages such as Python, I decided to write the whole project (the *lexer*, the **HOPPL** *parser* and the evaluation engine) from scratch in **Rust**. The decision rests on four concrete reasons.

### 1. Pattern Matching and Algebraic Data Types (ADTs) for the AST

An interpreter spends most of its time manipulating abstract syntax trees (ASTs) and evaluating recursive expressions.

- Rust **enums** model the language's expressions (operations, closures, `sample` calls, `observe` calls, etc.) directly.
- **Exhaustive pattern matching** (`match`) forces the evaluator to cover every AST case. If a new node or primitive is added, the compiler points out exactly which parts of the evaluator were left unupdated.

### 2. Type Safety and Compile-Time Error Detection

Unlike dynamically typed languages, where logic or memory errors only appear at runtime (sometimes midway through a long simulation), Rust's type system and *borrow checker* catch a good share of these problems before the program runs:

- **No null references:** using `Option<T>` and `Result<T, E>` forces explicit handling of the *Parser*'s syntax errors and the *Evaluator*'s semantic errors.
- **Memory safety without a garbage collector:** memory leaks and segmentation faults are avoided without paying the cost of garbage-collector pauses.

### 3. Performance in the Inference Algorithms

The implemented algorithms (*Sequential Monte Carlo*, *Metropolis-Hastings*, *Likelihood Weighting*) are computationally heavy:

- In **SMC**, for example, thousands of "particles" (execution traces) must be maintained, evaluated and cloned in parallel, with *resampling* happening constantly.
- By compiling to native machine code with *zero-cost abstractions*, Rust runs thousands of MCMC steps or SMC particles in a fraction of the time Python would take, with performance close to C or C++.

### 4. Trace Tracking

Implementing algorithms such as **Single-Site Metropolis-Hastings** requires maintaining a *Trace* that associates each `sample` call with its execution *Address*. Rust's *ownership* system, together with explicit data cloning, makes it simpler to keep track of these addresses without unexpected side effects when modifying the state of random variables.

---

## Technical Notes: Pure Functional CPS vs. CEK Machine

During the design of the project, and following a suggestion from the professor, I evaluated implementing the expression evaluator with pure functional **Continuation-Passing Style (CPS)**. In classic Lisp literature this is achieved by passing higher-order functions (*closures*) as continuations to pause and resume the control flow.

I ended up discarding pure CPS and using a **CEK Machine (Control, Environment, Continuation)** instead, which is the mathematical "defunctionalization" of CPS. The decision solves two concrete problems that arise when doing this in Rust:

1. **The cloning barrier (the `fork` function for SMC and MCMC):** this was the deciding factor. Algorithms such as Sequential Monte Carlo need to pause execution at every `observe`, **clone** the machine state into multiple particles and resume in parallel. In Rust, cloning an arbitrary closure hidden behind dynamic traits (e.g. `Box<dyn Fn>`) is hard, because the compiler does not know the size or contents of the captured environment at runtime. With a CEK machine, the "continuation" stops being an opaque function and becomes a concrete data structure (a vector of enums, `Vec<Instr>`), which can be cloned with a simple `#[derive(Clone)]`.

2. **Opaque types and TCO (Tail Call Optimization):** implementing pure CPS means building recursive return types and chaining closures. In Rust, dealing with the *lifetimes* of references inside nested closures considerably hurts the evaluator's readability and maintainability. Moreover, since Rust does not guarantee tail-call optimization, pure CPS with deep probabilistic recursion would end in a *stack overflow*. The CEK machine's explicit stack handles the control flow iteratively on the *heap*, avoiding that problem.

---

## Extras

This section covers things added beyond the assignment to make the project more complete.

### MCMC Convergence Diagnostics and Metrics (`src/stats.rs`)

To rigorously evaluate the quality of the chains produced by the inference algorithms and to ensure the statistical validity of the approximate results, the `stats.rs` module computes and reports three key diagnostic metrics at the end of a run.

#### 1. 95% Confidence Interval (95% CI)

Gives the $2.5\%$ and $97.5\%$ percentiles of the empirical marginal distribution obtained from the samples. It provides a high-density probability region that locates where the true value of the latent parameters is concentrated, at a standard level of statistical significance.

#### 2. Effective Sample Size Percentage (ESS%)

Because of the sequential, stochastic nature of algorithms such as Metropolis-Hastings, successive samples of the chain tend to be strongly autocorrelated. The **Effective Sample Size (ESS)** estimates how many independent, *uncorrelated* samples the actual trace contains:

$$\text{ESS} = \frac{N}{1 + 2 \sum_{k=1}^{\infty} \rho_k}$$

Where $N$ is the total sample size and $\rho_k$ is the autocorrelation at lag $k$.

- **ESS%:** the percentage ratio $(\text{ESS} / N) \times 100$. A low ESS% (e.g. $< 5\%$) warns the developer of strong correlation and *poor mixing*, suggesting the need to increase the chain length or tune the proposals.

#### 3. Acceptance Rate

A metric specific to MCMC algorithms (such as Single-Site MH) that measures the proportion of proposed states that were accepted out of the total number of iterations:

$$\text{Acceptance Rate} = \frac{\text{Accepted Proposals}}{\text{Total Iterations}}$$

- **Interpretation:** it allows tuning the step size of the proposal distributions. An excessively high rate indicates the algorithm is taking very small steps and exploring inefficiently, while a very low rate means most jumps are rejected, leaving the chain stuck in the same state.

### Parser (`src/parser/sexpr.rs`)

This project implements a complete parsing pipeline from scratch in **Rust** to process **S-expressions**, the classic syntax format of the Lisp/Clojure family. The parser translates plain-text source code directly into a typed, safe Abstract Syntax Tree (AST).

Unlike the implicit dynamic typing of the original Python implementation, the Rust version models the whole system with **Algebraic Data Types (ADTs)** using enums and pattern matching.

Parsing is divided into two main stages.

#### 1. The Tokenizer or Lexer (`tokenize`)

The tokenizer sequentially scans the source character stream and groups it into a vector of internal tokens defined by the `Token` enum:

- `Token::LParen`: an opening delimiter. Both parentheses `(` and brackets `[` are unified under this token.
- `Token::RParen`: a closing delimiter, unifying both `)` and `]`.
- `Token::StringLit(String)`: literal strings (e.g. `"result"`).
- `Token::Atom(String)`: identifiers, numbers and symbols (e.g. `+`, `x`, `42`, `3.14`).

**Lexer features:**

- **Whitespace and comments:** whitespace, tabs, newlines and commas `,` (which act as readability separators in Clojure) are ignored. Comments starting with a semicolon (`;`) are skipped entirely until the end of the line.
- **String and escape support:** strings enclosed in double quotes (`"..."`) support the standard escape sequences (`\n`, `\t`, `\\`, `\"`). If a string is left open, the lexer raises a precise syntax error instead of failing silently.

#### 2. The Recursive Descent Parser (`read_form`)

The parser consumes the token vector recursively and builds the AST, represented by the `Form` type:

```rust
pub enum Form {
    Symbol(String),  // Identifiers (variables, primitives, etc.)
    Int(i64),        // 64-bit integers
    Float(f64),      // 64-bit floats
    Bool(bool),      // Booleans (true/false)
    Str(String),     // Literal strings
    Nil,             // Null value (nil)
    List(Vec<Form>), // Compound/nested expressions
}
```

**Key parser mechanisms:**

- **Atom conversion (`atom`):** converts text tokens into specific `Form` variants. It first looks for keywords such as `true`, `false` and `nil`. If none match, it strictly attempts to parse them as 64-bit integers or 64-bit floats. If numeric parsing fails, they are safely classified as `Form::Symbol`.
- **Recursive list parsing:** upon detecting a `Token::LParen`, it opens a new list and recursively processes all sub-elements until it finds the matching `Token::RParen`.
- **Robust syntax error handling:** instead of panicking or returning inconsistent results, the parser detects imbalances and reports descriptive errors with clear indications of the problem (for example, parentheses or brackets that were opened but never closed).

#### Module Public API

The module exposes a clean interface to be consumed by the evaluator or the inference engines:

- `parse(text: &str) -> Result<Vec<Form>, String>`: processes a complete program and returns a list of all top-level forms (expressions) found.
- `parse_one(text: &str) -> Result<Form, String>`: used to process exactly one expression. Raises a friendly error if the text is empty or contains multiple loose top-level forms.
- `to_string(form: &Form) -> String`: the inverse function, which takes an AST node and renders it back as readable Clojure code (preserving the `.0` format for floats with no fractional part, ensuring type consistency).

### Static Detection of SMC Desynchronization

In the **Sequential Monte Carlo (SMC)** inference algorithm (or particle filter), all particles represent concurrent execution traces that must advance in a synchronized fashion. Specifically, every time the particles hit an `observe` instruction, they must stop in unison (a synchronization point) to evaluate the likelihood of the observed value, update their accumulated weights and take part in the coordinated process of multinomial *resampling*.

If a particle took an alternative path in which it does not execute an `observe` that the others do (or vice versa), the trace desynchronizes and breaks the mathematical consistency of the algorithm.

To avoid this risk, the project implements **two layers of defense**: a **static analysis** before execution and a **dynamic detection** at runtime.

#### 1. Static Analysis Prior to Execution

At the start of the main `run_smc` function, before initializing the evaluation machine or creating the particles, the program is parsed and its Abstract Syntax Tree (AST) structure is exhaustively inspected:

```rust
pub fn run_smc<R: Rng + ?Sized>(...) -> Result<Vec<RVal>, String> {
    // 1. Parse the source code into its AST representation (Form)
    let forms = parse(program)?;

    // 2. Run the static desynchronization check
    check_scm_safety(&forms)?;

    // ... rest of the SMC algorithm
}
```

The check consists of two main helper functions:

- **`check_scm_safety(forms: &[Form]) -> Result<(), String>`**:
  Recursively iterates over all top-level expressions of the source code, calling `check_form` on each. If any of them violates the structural safety rules, it immediately aborts the algorithm's startup and propagates a descriptive error message.

- **`check_form(form: &Form) -> Result<bool, String>`**:
  A recursive descent function that inspects the AST and has two goals:
  1. Return `Ok(true)` if the current expression or any of its sub-expressions contains an `observe` (to notify parent nodes of the presence of observations).
  2. Raise an `Err(String)` if it finds an `observe` in a syntactic context that violates deterministic synchronization.

##### Statically Detected Forbidden Patterns

1. **`observe` inside conditional branches (`if`):**

   ```clojure
   (if condition (observe (normal 0 1) 0.5) (sample (normal 0 1)))
   ```

   - **Why it is forbidden:** the `if` condition may depend on each particle's random stochastic state. If some particles evaluate the condition as `true` and others as `false`, some will execute the `observe` and others will not, immediately breaking the alignment of the SMC traces.
   - **Reported error:** *"SMC Static Analysis Error: Found an 'observe' statement inside an 'if' branch. SMC requires a deterministic observation flow. Please move the observation outside the conditional."*

2. **`observe` inside function definitions (`fn` or `defn`):**

   ```clojure
   (let [my-function (fn [x] (observe (normal x 1) 2.0))] ...)
   ```

   - **Why it is forbidden:** functions and closures can be stored, passed as arguments, or invoked dynamically an arbitrary number of times (or never) at runtime. It is therefore mathematically impossible to statically guarantee observation synchronization if the observations live inside a function.
   - **Reported error:** *"SMC Static Analysis Error: Found an 'observe' statement inside a 'fn' definition. Functions can be called dynamically, which breaks SMC synchronization guarantees."*

3. **Propagation through `let` blocks:**
   It tracks the presence of `observe` both in the values bound to variables and in the expressions that make up the `let` body, so that no instruction goes undetected.

---

#### 2. Dynamic Runtime Safeguard

As a complementary safety net, if an extremely complex dynamic execution flow manages to evade the static analysis and causes a real desynchronization of the particles at runtime, the `run_smc` function detects it immediately.

During the main advance loop, all particles are advanced in parallel until each one stops at its next synchronization point (returning a signal or `Msg` message):

```rust
for msg in messages {
    match msg {
        Msg::Observe(_addr, dist, y_obs, mut m) => {
            // Normal flow: all particles are synchronized at an 'observe'
            ...
        }
        // If any particle finished prematurely (Done) or stopped on another signal
        _ => return Err("SMC Desynchronization Error: Particles reached divergent execution states. All particles in Sequential Monte Carlo must encounter the exact same sequence of 'observe' statements.".into()),
    }
}
```

With this scheme, any desynchronization is detected before or during execution, with an immediate diagnostic instead of letting a simulation run that would silently give incorrect results.

### Extra Inference Algorithms

As an extra, the inference engine also covers 2 additional algorithms seen during the course. The same CEK virtual machine serves both, with no changes to its design.

#### 1. Black-Box Variational Inference (BBVI)

Unlike traditional Monte Carlo methods (MCMC/SMC), which approximate the posterior distribution through stochastic sampling, BBVI turns inference into a **mathematical optimization** problem.

- A family of parametrized guide distributions $q_\theta(x)$ is proposed for each probabilistic site.
- The goal is to find the parameters $\theta$ that minimize the divergence from the true distribution by maximizing the evidence lower bound (**ELBO**).
- To do this without requiring a global automatic differentiation engine, the algorithm uses the *Score Function Trick* (REINFORCE) and a natively implemented **Adam optimizer**, which adjusts the parameters through stochastic gradient descent/ascent.

> **Note:** this inference algorithm is covered in chapter 4 of the aforementioned book, on which the course is heavily based.

#### 2. Exact Enumeration

A **100% deterministic and exact** inference method. Instead of estimating by throwing random values, this algorithm exhaustively explores all possible universes or branches of the program.

- Each time execution reaches a `sample` instruction, the virtual machine "clones" (forks) itself for every possible value the distribution can take, exploring all paths in parallel and computing their exact probability via Bayes' rule.
- **Intrinsic limitation:** exact enumeration requires the probabilistic variables to have **finite support** (only bounded discrete distributions, such as Bernoulli or Categorical). If you try to enumerate a continuous distribution (such as the Normal, which has infinitely many possible outcomes), the engine raises a controlled error to avoid a combinatorial explosion and memory exhaustion.

> **Note:** unlike **BBVI**, this inference algorithm is not explicitly mentioned in the book, but it was covered in class. This is due to its limited applicability in real-world cases.

### Interactive Inference Debugger (TUI)

A terminal debugger (`src/debugger/`, built with **ratatui** + **crossterm**) was implemented that lets you run any HOPPL model step by step instead of end to end:

```bash
cargo run -- debug <file.hoppl> <algorithm>
```

The TUI is organized into panels (`src/debugger/render.rs`):

| Panel | Content |
|---|---|
| Header | Current state |
| **Model** | Source code of the program being debugged (so you always see which model is running without going back to the file) |
| **Current** | The specific state of the active engine |
| **Event log** | History of steps |
| Footer | Controls |

#### The Problem: Not All Algorithms "Pause" the Same Way

A naive debugger that simply pauses the CEK machine at every `sample`/`observe`/`factor` is enough for **Likelihood Weighting** (it is literally a single linear trace), but it does not describe the real execution of the other four algorithms:

- **Exact Enumeration** is not a trace but a tree: at each `sample`, one branch forks for every value of the finite support.
- **SMC** advances N particles in lockstep, synchronized at every `observe`, with resampling between synchronizations: there is no single trace to pause.
- **SSMH** does not pause at effects: each iteration re-runs the entire program, proposing a new value at one address and accepting/rejecting via Metropolis-Hastings.
- **BBVI** is gradient optimization over batches of traces: there are no addresses to pause at, only training iterations.

Therefore, instead of a single generic stepper, `src/debugger/engine/` defines one "step" engine per algorithm (`lw.rs`, `enumeration.rs`, `smc.rs`, `ssmh.rs`, `bbvi.rs`), dispatched through a closed `enum Engine` with exhaustive `match`. This is the same preference for **enums + pattern matching** over `Box<dyn Trait>` already explained in the [CPS vs. CEK](#technical-notes-pure-functional-cps-vs-cek-machine) section. Each engine reuses the real logic of the corresponding algorithm in `src/inference/` (exposed as `pub(crate)`) instead of reimplementing it, so debug mode never diverges from normal execution mode.

What "one step" (`s`) means in each engine:

| Algorithm | One step... |
|---|---|
| Likelihood Weighting | Advances the next effect (`sample`/`observe`/`factor`) of the trace. |
| Exact Enumeration | At each `sample`, you choose with `↑`/`↓` which value of the finite support to explore; sibling branches are left on a pending stack to be explored later. |
| SMC | Runs a full synchronization round: all particles up to the next `observe` (or the end), with multinomial resampling and the round's ESS. |
| Single-Site MH | Runs one full Metropolis-Hastings iteration: proposes a new value at a random address and shows the `log_alpha` and the accept/reject outcome. |
| BBVI | Runs one full Adam optimization step over a batch of traces, showing the ELBO and the variational parameters `θ`. |

#### What Is Shown at the End (`Done`)

Upon finishing, each engine reports the same estimate that the non-interactive mode (`cargo run -- <file> <algorithm>`) would report, computed with the same `src/stats.rs` functions (`sample_mean_std_err`, `mcmc_mean_std_err_ess`, `ci95_margin`), reused through a common helper (`posterior_summary_lines` in `src/debugger/engine/mod.rs`) so the four panels do not diverge from each other or from the CLI mode:

| Algorithm | What it reports at the end |
|---|---|
| SMC | Estimated mean ± standard error and 95% CI, over the final particles. |
| Single-Site MH | Mean, 95% CI and autocorrelation-adjusted ESS, over the post-warmup chain. |
| BBVI | Posterior mean (via the optimized guide) ± standard error and 95% CI, over the last batch of traces. |
| Exact Enumeration | Full P(value) table plus the mean weighted by the exact PMF (when the result is numeric); more informative than a mean alone since it is not an approximation. |
| Likelihood Weighting | Only the result and the `log_w` of that single trace. Unlike the other four, the LW debug engine pauses a single linear trace instead of running the N particles of the non-interactive mode, so there is no posterior mean to average. |

If the model's result is not numeric (for example, it returns a boolean), a per-value frequency breakdown is shown instead of the mean.

#### Controls

| Key | Action |
|---|---|
| `s` | Advance one step. |
| `c` | Continue automatically until the next breakpoint or the end (or, in SSMH/BBVI, until the configured step budget, since an MCMC chain or an optimization loop does not end on its own). |
| `b` | Toggle a breakpoint at the current address (not applicable in BBVI, which has no addresses to pause at). |
| `↑` / `↓` | Choose the highlighted branch (only has an effect in Exact Enumeration). |
| `←` / `→` | Navigate backward/forward through the already-explored history (read-only). |
| `q` / `Esc` | Quit. |

### Future Work: Toward a Probabilistic Experimentation Platform

Two lines of future work had been identified for the project, beyond what the assignment asked for. Both are now implemented:

> **Note:** the `factor` soft-conditioning operator (originally listed here as a future feature) has already been implemented. See [Tutorial, section 7](#7-soft-conditioning-with-factor) and demo 7 (`cargo run -- 7`).
>
> **Note:** the inference debugger and trace visualization (originally item 1 of this section) has also been implemented. See [Interactive Inference Debugger (TUI)](#interactive-inference-debugger-tui) and mode 5 of [Usage](#5-debug-a-hoppl-model-step-by-step-debug-mode).
