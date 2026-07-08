/*

Módulo para la implementacion de funciones deterministas y distribuciones del lenguaje que viven en el entorno global
de las máquinas de ejecución en forma de HashMap que vincula el simbolo con el procedimiento correspondiete.

*/

use std::{collections::HashMap};
use ndarray::{Array2};
use crate::parser::distribution::{make_distribution};
use crate::parser::value::RVal;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HashKey {
    Int(i64),
    Bool(bool),
    Str(String),
}
 
impl HashKey {
    pub fn from_rval(v: &RVal) -> Result<Self, String> {
        match v {
            RVal::Int(i) => Ok(HashKey::Int(*i)),
            RVal::Float(f) => Ok(HashKey::Int(*f as i64)),
            RVal::Bool(b) => Ok(HashKey::Bool(*b)),
            RVal::Str(s) => Ok(HashKey::Str(s.clone())),
            other => Err(format!("Type error: expected a valid hash-map key (Int, Float, Bool, or Str), but received: {:?}", other)),
        }
    }
}
 
impl std::fmt::Display for HashKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashKey::Int(i) => write!(f, "{i}"),
            HashKey::Bool(b) => write!(f, "{b}"),
            HashKey::Str(s) => write!(f, "{s}"),
        }
    }
}

// Tipo de funcion primitiva determinística
pub type PrimFn = Box<dyn Fn(&[RVal]) -> Result<RVal, String> + Send + Sync>;

// Tabla de primitivas determinísticas equivalente a la tabla de primitivas de Python. Se usa para construir el entorno inicial.

pub fn make_primitives() -> HashMap<&'static str, PrimFn> {
    let mut m: HashMap<&'static str, PrimFn> = HashMap::new();
 
    // aritmética
    m.insert("+",    Box::new(prim_add));
    m.insert("-",    Box::new(prim_sub));
    m.insert("*",    Box::new(prim_mul));
    m.insert("/",    Box::new(prim_div));
    m.insert("sqrt", Box::new(|a| Ok(RVal::Float(to_f64(&a[0])?.sqrt()))));
    m.insert("exp",  Box::new(|a| Ok(RVal::Float(to_f64(&a[0])?.exp()))));
    m.insert("log",  Box::new(|a| Ok(RVal::Float(to_f64(&a[0])?.ln()))));
    m.insert("pow",  Box::new(|a| Ok(smart_num(to_f64(&a[0])?.powf(to_f64(&a[1])?)))));
    m.insert("abs",  Box::new(|a| Ok(smart_num(to_f64(&a[0])?.abs()))));
    m.insert("floor",Box::new(|a| Ok(RVal::Int(to_f64(&a[0])?.floor() as i64))));
    m.insert("ceil", Box::new(|a| Ok(RVal::Int(to_f64(&a[0])?.ceil() as i64))));
    m.insert("tanh", Box::new(|a| Ok(RVal::Float(to_f64(&a[0])?.tanh()))));
    m.insert("max",  Box::new(|a| {
        let v: Result<Vec<f64>, _> = a.iter().map(to_f64).collect();
        Ok(smart_num(v?.iter().cloned().fold(f64::NEG_INFINITY, f64::max)))
    }));
    m.insert("min",  Box::new(|a| {
        let v: Result<Vec<f64>, _> = a.iter().map(to_f64).collect();
        Ok(smart_num(v?.iter().cloned().fold(f64::INFINITY, f64::min)))
    }));
    m.insert("mod", Box::new(|a| {
        let divisor = to_f64(&a[1])?;
        if divisor == 0.0 {
            Err("Arithmetic error: modulo by zero".into())
        } else {
            Ok(smart_num(to_f64(&a[0])? % divisor))
        }
    })); 
    // comparación y lógica
    m.insert("=",    Box::new(prim_eq));
    m.insert("==",   Box::new(prim_eq));
    m.insert("!=",   Box::new(|a| Ok(RVal::Bool(a[0] != a[1]))));
    m.insert("<",    Box::new(prim_lt));
    m.insert(">",    Box::new(prim_gt));
    m.insert("<=",   Box::new(prim_lte));
    m.insert(">=",   Box::new(prim_gte));
    m.insert("and",  Box::new(prim_and));
    m.insert("or",   Box::new(prim_or));
    m.insert("not",  Box::new(prim_not));
 
    // estructuras de datos
    m.insert("vector",   Box::new(prim_vector));
    m.insert("list",     Box::new(prim_vector)); // alias
    m.insert("hash-map", Box::new(prim_hash_map));
    m.insert("get",      Box::new(prim_get));
    m.insert("put",      Box::new(prim_put));
    m.insert("assoc",    Box::new(prim_put));    // alias
    m.insert("first",    Box::new(prim_first));
    m.insert("second",   Box::new(prim_second));
    m.insert("last",     Box::new(prim_last));
    m.insert("rest",     Box::new(prim_rest));
    m.insert("nth",      Box::new(prim_nth));
    m.insert("conj",     Box::new(prim_conj));
    m.insert("cons",     Box::new(prim_cons));
    m.insert("append",   Box::new(prim_append));
    m.insert("concat",   Box::new(prim_concat));
    m.insert("count",    Box::new(prim_count));
    m.insert("empty?",   Box::new(prim_empty));
    m.insert("peek",     Box::new(prim_peek));
    m.insert("range",    Box::new(prim_range));
    m.insert("vector?",  Box::new(prim_is_vector));
    m.insert("map?",     Box::new(prim_is_map));
    m.insert("number?",  Box::new(prim_is_number));
 
    // matrices
    m.insert("mat-mul",       Box::new(prim_mat_mul));
    m.insert("mat-add",       Box::new(prim_mat_add));
    m.insert("mat-transpose", Box::new(prim_mat_transpose));
    m.insert("mat-tanh",      Box::new(prim_mat_tanh));
    m.insert("mat-relu",      Box::new(prim_mat_relu));
    m.insert("mat-repmat",    Box::new(prim_mat_repmat));

    //  Constructores de distribuciones ( Equivalente a S.update(DISTRIBUTIONS) de Python)
    // Cada nombre de distribucion toma sus argumentos y devuelve un RVal::Dist con la distribucion correspondiente
    // Notar que hay algunas distribuciones que tiene aliases, como por ejemplo bernoulli con flip.
        for name in &[
        "normal", "log-normal", "beta", "gamma", "exponential",
        "uniform-continuous", "uniform", "poisson", "bernoulli", "flip",
        "discrete", "categorical", "uniform-discrete", "dirichlet",
    ] {
        let name_owned = *name;
        m.insert(name_owned, Box::new(move |args: &[RVal]| {
    // Si la distribución es "discrete" o "dirichlet", extrae los datos de la lista
    let dist = if name_owned == "discrete" || name_owned == "categorical" || name_owned == "dirichlet" {
        // Extrae el vector de RVal::List y conviértelo a Vec<f64>
        if let RVal::List(v) = &args[0] {
            let nums: Result<Vec<f64>, _> = v.iter().map(to_f64).collect();
            make_distribution(name_owned, &nums?)?
        } else {
            // Caso donde pasan los números como argumentos individuales
            let nums: Result<Vec<f64>, _> = args.iter().map(to_f64).collect();
            make_distribution(name_owned, &nums?)?
        }
    } else {
        // Caso normal (normal, log-normal, etc.)
        let nums: Result<Vec<f64>, _> = args.iter().map(to_f64).collect();
        make_distribution(name_owned, &nums?)?
    };
    Ok(RVal::Dist(dist))
}));
    }
    m
}

// Funcion auxiliar que sirve para detectar si es una función primitiva.
pub fn is_primitive(name: &str) -> bool {
    make_primitives().contains_key(name)
}

// Helpers de conversion equivalente a los de python
// Bool se convierte igual que python: true -> 1, false -> 0
fn to_f64(v: &RVal) -> Result<f64, String> {
    match v {
        RVal::Int(i) => Ok(*i as f64),
        RVal::Float(f) => Ok(*f),
        RVal::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        other => Err(format!("Type error: expected a numeric value (Int, Float, or Bool), but received: {}", other)),
    }
}



// Convierte un RVal a u64 para operaciones con indices
fn to_i64(v: &RVal) -> Result<i64, String> {
    match v {
        RVal::Int(i) => Ok(*i),
        RVal::Float(f) => Ok(*f as i64),
        RVal::Bool(b) => Ok(if *b { 1 } else { 0 }),
        other => Err(format!("Type error: expected an integer value, but received: {}", other)),
    }
}


// Convierte a Array2<f64>. Acepta Matrix directamente, o una List de Lists
// (equivalente a _to_mat del Python).

fn to_matrix(v: &RVal) -> Result<Array2<f64>, String> {
    match v {
        RVal::Matrix(m) => Ok(m.clone()),
        RVal::List(rows) => {
            if rows.is_empty() {
                return Ok(Array2::zeros((0, 0)));
            }
            let ncols = match &rows[0] {
                RVal::List(r) => r.len(),
                _ => {
                    // vector 1-D -> columna
                    let data: Result<Vec<f64>, _> = rows.iter().map(to_f64).collect();
                    let data = data?;
                    let n = data.len();
                    return Ok(Array2::from_shape_vec((n, 1), data)
                        .map_err(|e| format!("Shape error in matrix conversion: {}", e))?);
                }
            };
            let mut flat = Vec::with_capacity(rows.len() * ncols);
            for row in rows {
                if let RVal::List(r) = row {
                    for x in r {
                        flat.push(to_f64(x)?);
                    }
                } else {
                    return Err("Type error in matrix conversion: expected a List of Lists".into());
                }
            }
            Array2::from_shape_vec((rows.len(), ncols), flat).map_err(|e| format!("Shape error in matrix conversion: {}", e))
        }
        other => Err(format!("Type error: expected a Matrix or a List of Lists, but received: {}", other)),
    }
}
 
fn array2_to_rval(m: Array2<f64>) -> RVal {
    RVal::Matrix(m)
}


// Implementación de funciones primitivas determinísticas

// -- Aritmetica --

fn prim_add(args: &[RVal]) -> Result<RVal, String> {
    // Soporta matrices (como numpy)
    if args.iter().any(|a| matches!(a, RVal::Matrix(_))) {
        let mut acc = to_matrix(&args[0])?;
        for a in &args[1..] {
            acc = acc + to_matrix(a)?;
        }
        return Ok(array2_to_rval(acc));
    }
    let sum: f64 = args.iter().map(to_f64).collect::<Result<Vec<_>, _>>()?.iter().sum();
    Ok(smart_num(sum))
}
 
fn prim_sub(args: &[RVal]) -> Result<RVal, String> {
    if args.len() == 1 {
        return Ok(smart_num(-to_f64(&args[0])?));
    }
    let mut out = to_f64(&args[0])?;
    for a in &args[1..] {
        out -= to_f64(a)?;
    }
    Ok(smart_num(out))
}

fn prim_mul(args: &[RVal]) -> Result<RVal, String> {
    let mut out = to_f64(&args[0])?;
    for a in &args[1..] {
        out *= to_f64(a)?;
    }
    Ok(smart_num(out))
}
 
fn prim_div(args: &[RVal]) -> Result<RVal, String> {
    if args.len() == 1 {
        let val = to_f64(&args[0])?;
        if val == 0.0 {
            return Err("Arithmetic error: division by zero".into());
        }
        return Ok(RVal::Float(1.0 / val));
    }

    let mut out = to_f64(&args[0])?;
    for a in &args[1..] {
        let divisor = to_f64(a)?;
        if divisor == 0.0 {
            return Err("Arithmetic error: division by zero".into());
        }
        out /= divisor;
    }
    Ok(RVal::Float(out))
}


/// Si el valor es un entero exacto, devuelve RVal::Int; si no, RVal::Float.
/// Replica el comportamiento de Python donde int+int sigue siendo int.
fn smart_num(x: f64) -> RVal {
    if x.fract() == 0.0 && x.abs() < i64::MAX as f64 {
        RVal::Int(x as i64)
    } else {
        RVal::Float(x)
    }
}

// -- Comparaciones --

fn prim_eq(args: &[RVal]) -> Result<RVal, String> {
    let (a, b) = (&args[0], &args[1]);
    match (a, b) {
        (RVal::Matrix(ma), RVal::Matrix(mb)) => Ok(RVal::Bool(ma == mb)),
        _ => Ok(RVal::Bool(a == b)),
    }
}

fn prim_lt(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(to_f64(&args[0])? < to_f64(&args[1])?))
}

fn prim_gt(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(to_f64(&args[0])? > to_f64(&args[1])?))
}

fn prim_lte(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(to_f64(&args[0])? <= to_f64(&args[1])?))
}

fn prim_gte(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(to_f64(&args[0])? >= to_f64(&args[1])?))
}

fn prim_and(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(args.iter().all(|a| to_bool(a))))
}

fn prim_or( args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(args.iter().any(|a| to_bool(a))))
}

fn prim_not(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(!to_bool(&args[0])))
}

fn to_bool(v: &RVal) -> bool {
    match v {
        RVal::Bool(b) => *b,
        RVal::Nil => false,
        RVal::Int(0) => false,
        _ => true,
    }
}


// -- Estructuras de datos --

fn prim_vector(args: &[RVal]) -> Result< RVal, String> {
    Ok(RVal::List(args.to_vec()))
}

fn prim_hash_map(args: &[RVal]) -> Result< RVal, String> {
    if args.len() % 2 != 0 {
        return Err("Arity error: 'hash-map' expects an even number of arguments to form key-value pairs".into());
    }
    let mut map = HashMap::new();
    for i in (0..args.len()).step_by(2) {
        let key = HashKey::from_rval(&args[i])?;
        let value = args[i + 1].clone();
        map.insert(key, value);
    }
    Ok(RVal::Map(map))
}

fn prim_get(args: &[RVal]) -> Result< RVal, String> {
    let coll = &args[0];
    let key = &args[1];
    let default = args.get(2).cloned().unwrap_or(RVal::Nil);
    match coll {
        RVal::Map(m) => {
            let k = HashKey::from_rval(key)?;
            Ok(m.get(&k).cloned().unwrap_or(default))
        }
        RVal::List(l) => {
            let idx = to_i64(key)?;
            if idx < 0 || (idx as usize) >= l.len() {
                Ok(default)
            } else {
                Ok(l[idx as usize].clone())
            }
        }
        RVal::Matrix(m) => {
            let idx = to_i64(key)? as usize;
            // indexar por fila devuelve una List
            let row: Vec<RVal> = m.row(idx).iter().map(|&x| RVal::Float(x)).collect();
            Ok(RVal::List(row))
        }
        other => Err(format!("Type error in 'get': expected a collection (Map, List, or Matrix), but received: {}", other))
        }
    }


fn prim_put(args: &[RVal]) -> Result< RVal, String> {
    let coll = &args[0];
    let key = &args[1];
    let value = args[2].clone();
    match coll {
        RVal::Map(m) => {
            let mut out = m.clone();
            out.insert(HashKey::from_rval(key)?, value);
            Ok(RVal::Map(out))

        }
        RVal::List(v) => {
            let idx = to_i64(key)? as usize;
            let mut out = v.clone();
            out[idx] = value;
            Ok(RVal::List(out))
        }
        other => Err(format!("Type error in 'put'/'assoc': expected a collection (Map or List), but received: {}", other)),
    }
}

fn prim_first(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => v.first().cloned().ok_or("Value error in 'first': cannot get the first element of an empty List".into()),
        other => Err(format!("Type error in 'first': expected a List, but received: {}", other)),
    }
}

fn prim_second(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => v.get(1).cloned().ok_or("Value error in 'second': the List does not have a second element".into()),
        other => Err(format!("Type error in 'second': expected a List, but received: {}", other)),
    }
}

fn prim_last(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => v.last().cloned().ok_or("Value error in 'last': cannot get the last element of an empty List".into()),
        other => Err(format!("Type error in 'last': expected a List, but received: {}", other)),
    }
}

fn prim_rest(args: &[RVal]) -> Result< RVal, String> {
    match &args[0] {
        RVal::List(v) => Ok(RVal::List(v[1..].to_vec())),
        other => Err(format!("Type error in 'rest': expected a List, but received: {}", other)),
    }
}

fn prim_nth(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => {
            let i = to_i64(&args[1])? as usize;
            v.get(i).cloned().ok_or(format!("Index error in 'nth': index {} is out of bounds", i))
        }
        other => Err(format!("Type error in 'nth': expected a List, but received: {}", other)),
    }
}

fn prim_conj(args: &[RVal]) -> Result< RVal, String> {
    let mut out = match &args[0] {
        RVal::List(v) => v.clone(),
        other => return Err(format!("Type error in 'conj': expected a List as the base collection, but received: {}", other)),
    };
    out.extend_from_slice(&args[1..]);
    Ok(RVal::List(out))
}

fn prim_cons(args: &[RVal]) -> Result < RVal, String> {
    let x = args[0].clone();
    let coll = match &args[1] {
        RVal::List(v) => v.clone(),
        other => return Err(format!("Type error in 'cons': expected a List as the base collection, but received: {}", other)),
    };
    let mut out = vec![x];
    out.extend(coll);
    Ok(RVal::List(out))
}

fn prim_append(args: &[RVal]) -> Result<RVal, String> {
    let mut out = match &args[0] {
        RVal::List(v) => v.clone(),
        other => return Err(format!("Type error in 'append': expected a List as the base collection, but received: {}", other)),
    };
    out.extend_from_slice(&args[1..]);
    Ok(RVal::List(out))
}
 
fn prim_concat(args: &[RVal]) -> Result<RVal, String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            RVal::List(v) => out.extend_from_slice(v),
            other => return Err(format!("Type error in 'concat': all arguments must be Lists, but received: {}", other)),
        }
    }
    Ok(RVal::List(out))
}
 
fn prim_count(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => Ok(RVal::Int(v.len() as i64)),
        RVal::Map(m) => Ok(RVal::Int(m.len() as i64)),
        RVal::Str(s) => Ok(RVal::Int(s.len() as i64)),
        other => Err(format!("Type error in 'count': expected a collection (List, Map, or String), but received: {}", other)),
    }
}
 
fn prim_empty(args: &[RVal]) -> Result<RVal, String> {
    match &args[0] {
        RVal::List(v) => Ok(RVal::Bool(v.is_empty())),
        RVal::Map(m) => Ok(RVal::Bool(m.is_empty())),
        other => Err(format!("Type error in 'empty?': expected a collection (List or Map), but received: {}", other)),
    }
}
 
fn prim_peek(args: &[RVal]) -> Result<RVal, String> {
    prim_last(args) // en Clojure, peek en vector = last
}
 
fn prim_range(args: &[RVal]) -> Result<RVal, String> {
    let (start, end_, step) = match args.len() {
        1 => (0i64, to_i64(&args[0])?, 1i64),
        2 => (to_i64(&args[0])?, to_i64(&args[1])?, 1i64),
        3 => (to_i64(&args[0])?, to_i64(&args[1])?, to_i64(&args[2])?),
        _ => return Err("Arity error in 'range': expected 1, 2, or 3 arguments".into()),
    };
    let v: Vec<RVal> = (start..end_).step_by(step as usize).map(RVal::Int).collect();
    Ok(RVal::List(v))
}
 
fn prim_is_vector(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(matches!(&args[0], RVal::List(_))))
}
 
fn prim_is_map(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(matches!(&args[0], RVal::Map(_))))
}
 
fn prim_is_number(args: &[RVal]) -> Result<RVal, String> {
    Ok(RVal::Bool(matches!(&args[0], RVal::Int(_) | RVal::Float(_))))
}
 
// --- operaciones matriciales ---
 
fn prim_mat_mul(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    let b = to_matrix(&args[1])?;
    let c = a.dot(&b);
    Ok(array2_to_rval(c))
}
 
fn prim_mat_add(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    let b = to_matrix(&args[1])?;
    Ok(array2_to_rval(a + b))
}
 
fn prim_mat_transpose(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    Ok(array2_to_rval(a.t().to_owned()))
}
 
fn prim_mat_tanh(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    Ok(array2_to_rval(a.mapv(f64::tanh)))
}
 
fn prim_mat_relu(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    Ok(array2_to_rval(a.mapv(|x| x.max(0.0))))
}
 
/// mat-repmat: equivalente a np.tile(a, (r, c)).
fn prim_mat_repmat(args: &[RVal]) -> Result<RVal, String> {
    let a = to_matrix(&args[0])?;
    let r = to_i64(&args[1])? as usize;
    let c = to_i64(&args[2])? as usize;
    let (nrows, ncols) = a.dim();
    let mut result = Array2::zeros((nrows * r, ncols * c));
    for ri in 0..r {
        for ci in 0..c {
            result
                .slice_mut(ndarray::s![
                    ri * nrows..(ri + 1) * nrows,
                    ci * ncols..(ci + 1) * ncols
                ])
                .assign(&a);
        }
    }
    Ok(array2_to_rval(result))
}