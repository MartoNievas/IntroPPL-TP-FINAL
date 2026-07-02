/* 

Forms are represented as plain Python data:
  - Symbol (a str subclass)        identifiers:  x, +, sample, mat-mul
  - int / float                    numbers
  - bool                           true / false
  - str                           "double-quoted strings"
  - list                           compound forms: (op e1 e2 ...)

Square brackets are read as ordinary lists, i.e. ``[x 1]`` == ``(x 1)``;
the FOPPL/HOPPL ``let`` desugarer interprets the binding list itself.
Comments run from ``;`` to end of line.

*/

// Form: AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    Symbol(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
    List(Vec<Form>),
}

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_string(self))
    }
}


/* 

Token: tipo interno del tokenizador 

*/

#[derive(Debug, Clone)]
enum Token {
    LParen,
    RParen,
    StringLit(String),

    Atom(String),
}


/*
    tokenize
    Convierte el texto fuerte en una secuencia de tokens
    Es equivalente al tokenize de python
*/ 

pub fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let mut chars = text.chars().peekable();
    let mut tokens : Vec<Token> = Vec::new();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' | ',' => {
                chars.next();
            },
            ';' => {
                // iteramos hasta encontrar un salto de linea
                while chars.next_if(|&c| c != '\n').is_some() {}
            }
            '['  | '('  => {
                chars.next(); tokens.push(Token::LParen);
            }
            ']' | ')' => {
                chars.next(); tokens.push(Token::RParen);
            }

            // String literal
            '"' => {
                chars.next(); // Consumimos comilla de apertura
                let mut buffer = String::new();
                loop {
                    match chars.next() {
                        None => return Err("Unterminated string literal".into()),
                        Some('"') => break, // llegamos al final del string literal
                        Some('\\') => match chars.next() { // char escapado
                            Some('n') => buffer.push('\n'),
                            Some('t') => buffer.push('\t'),
                            Some('\\') => buffer.push('\\'),
                            Some('"') => buffer.push('"'),
                            Some(c) => buffer.push(c),
                            None => return Err("Unterminated escape".into()),
                        },
                        Some(c) => buffer.push(c),
                    }
                }
                tokens.push(Token::StringLit(buffer));
            }

            // Atomo
            _ => {

                let mut buf = String::new();
                while let Some(&c) = chars.peek() {
                    if matches!(c, ' '|'\t'|'\n'|'\r'|','|'('|')'|'['|']'|';'|'"') {        
                        break;
                    }
                
                buf.push(c);
                chars.next();
                }
                tokens.push(Token::Atom(buf))
            }   
        }
    }
    Ok(tokens)
}


/*

    atom: convierte un token Atom en el Form adecuado
    es equivalente a la version de python
*/

fn atom(s: &str) -> Form {
    match s {
        "true" => return Form::Bool(true),
        "false" => return Form::Bool(false),
        "nil" => return Form::Nil,
        _ => {},
    }
    if let Ok(i) = s.parse::<i64>() {
        return Form::Int(i);
    }

    if let Ok(f) = s.parse::<f64>() {
        return Form::Float(f);
    }

    Form::Symbol(s.to_string())
}

/*

read_form: parser recursivo
equivalente a _read de python

*/

fn read_form(tokens: &[Token], pos: usize) -> Result<(Form, usize), String> {
    if pos >= tokens.len() {
        return Err("unexpected end of input".to_string());
    }

    match &tokens[pos] {
        Token::LParen => {
            let mut forms = Vec::new();
            let mut cur = pos + 1;
            
            loop {

                if cur >= tokens.len() {
                    return Err("Missing closing parenthesis".to_string());
                }

                if matches!(tokens[cur], Token::RParen) {
                    return Ok((Form::List(forms), cur + 1));                
                }
                
                let (sub, next) = read_form(tokens, cur)?;
                forms.push(sub);
                cur = next;

            }
        }

        Token::RParen => {
            Err("Unexpected )".to_string())
        }

        Token::StringLit(s) => {
            Ok((Form::Str(s.clone()), pos + 1))
        }

        Token::Atom(s) => {
            Ok((atom(s), pos + 1))
        }

    }

}


/*

Ahora definimos la API publica del parser del lenguaje con la funciones:
parse / parse_one / to_string

*/

/*

Parsea el texto fuente y devuelve una lista de todas las formas de nivel superior
Equivalente a `parse(texto)` de python

*/

pub fn parse(text: &str) -> Result<Vec<Form>, String> {
    let tokens = tokenize(text)?;
    let mut forms = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        let (form, next) = read_form(&tokens, pos)?;
        forms.push(form);
        pos = next;
    }

    Ok(forms)
}

/*

Parsea el texto que contiene exactamente una forma de nivel superior
Equivalente a `parse_one(text) de python`

*/

pub fn parse_one(text: &str) -> Result<Form, String> {
    let forms = parse(text)?;
    match  forms.len() {
        1 => Ok(forms.into_iter().next().unwrap()),
        n => Err(format!("Expected exactly one form, got {n}")),
    }
}


/*

Renderiza un Form de vuelta a texto fuente (aproximado).
Equivalente a `to_string(form)` de python

*/

pub fn to_string(form: &Form) -> String {
    match form {
        Form::Bool(true) => "true".to_string(),
        Form::Bool(false) => "false".to_string(),
        Form::Nil => "nil".to_string(),
        Form::Symbol(s) => s.clone(),
        Form::Str(s) => format!("\"{s}\""),
        Form::Int(i) => i.to_string(),
        Form::Float(f) => {
            // Si es un numero entero exacto, mostramos con .0 para que siga siendo float

            if f.fract() == 0.0 {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        Form::List(forms) => {
            let inner: Vec<String> = forms.iter().map(to_string).collect();
            format!("({})",inner.join(" "))
        }
    }
}

