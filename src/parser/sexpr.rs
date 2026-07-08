/* 

Módulo que se encarga de parsear s-expresiones y crear el AST correspondiente.
Tambien se implementa to_string que toma el AST y hace el camino inverso.

*/

// ListType: nos ayuda a distinguir si tenemos que poner brackest o parentesis en el to_string
#[derive(Debug, Clone, PartialEq)]
pub enum ListType {
    Paren, // ()
    Bracket, // []
}

// Form: AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    Symbol(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
    List(Vec<Form>, ListType),
}

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_string(self))
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
        n => Err(format!("Parsing error: Expected exactly one top-level form, but found {}. If you have multiple expressions, wrap them in a block like (let [...] ...).", n)),
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
        Form::List(forms, list_type) => {
            let inner: Vec<String> = forms.iter().map(to_string).collect();
            let content = inner.join(" ");
            match list_type {
                ListType::Paren => {
                    format!("({})",content)
                }
                ListType::Bracket => {
                    format!("[{}]", content)
                }
            }
            
        }
    }
}


/* Token: tipo interno del tokenizador 

*/

#[derive(Debug, Clone)]
enum Token {
    LParen,
    RParen,
    LBracket,
    RBracket,
    StringLit(String),

    Atom(String),
}


/*
    tokenize
    Convierte el texto fuerte en una secuencia de tokens
    Es equivalente al tokenize de python
*/ 

fn tokenize(text: &str) -> Result<Vec<Token>, String> {
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
            '('  => {
                chars.next(); tokens.push(Token::LParen);
            }
            '[' => {
                chars.next(); tokens.push(Token::LBracket);
            }
            ')' => {
                chars.next(); tokens.push(Token::RParen);
            }
            ']' => {
                chars.next(); tokens.push(Token::RBracket);
            }

            // String literal
            '"' => {
                chars.next(); // Consumimos comilla de apertura
                let mut buffer = String::new();
                loop {
                    match chars.next() {
                        None => return Err("Syntax error: Unterminated string literal. A string was opened with '\"' but never closed.".into()),
                        Some('"') => break, // llegamos al final del string literal
                        Some('\\') => match chars.next() { // char escapado
                            Some('n') => buffer.push('\n'),
                            Some('t') => buffer.push('\t'),
                            Some('\\') => buffer.push('\\'),
                            Some('"') => buffer.push('"'),
                            Some(c) => buffer.push(c),
                            None => return Err("Syntax error: Unterminated escape sequence at the end of a string literal.".into()),
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
        return Err("Syntax error: Unexpected end of input while parsing. Check for unclosed parentheses or brackets.".to_string());
    }

    match &tokens[pos] {
        Token::LParen => {
            let mut forms = Vec::new();
            let mut cur = pos + 1;
            
            loop {

                if cur >= tokens.len() {
                    return Err("Syntax error: Missing closing parenthesis ')'. An opened list was never closed.".to_string());
                }

                if matches!(tokens[cur], Token::RParen) {
                    return Ok((Form::List(forms, ListType::Paren), cur + 1));                
                }
                
                let (sub, next) = read_form(tokens, cur)?;
                forms.push(sub);
                cur = next;

            }
        }

        Token::LBracket => {
            let mut forms : Vec<Form> = Vec::new();
            let mut cur = pos + 1;
            loop {

                if cur >= tokens.len() {
                    return Err("Syntax error: Missing closing bracket ']'. An opened list was never closed.".to_string());
                }

                if matches!(tokens[cur], Token::RBracket) {
                    return Ok((Form::List(forms, ListType::Bracket), cur + 1));
                }

                let (sub, next) = read_form(tokens, cur)?;
                forms.push(sub);
                cur = next;
            }

        }

        Token::RParen => {
            Err("Syntax error: Unexpected closing parenthesis ')'. Found a closing delimiter without a matching opening delimiter.".to_string())
        }

        Token::RBracket => {
            Err("Syntax error: Unexpected closing bracket ']'. Found a closing delimiter without a matching opening delimiter.".to_string())
        }

        Token::StringLit(s) => {
            Ok((Form::Str(s.clone()), pos + 1))
        }

        Token::Atom(s) => {
            Ok((atom(s), pos + 1))
        }

    }

}
