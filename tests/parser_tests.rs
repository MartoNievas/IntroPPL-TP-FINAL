use PPL_TP_FINAL::parser::sexpr::*;

#[cfg(test)]
mod tests_parser {
    use super::*;
 
    #[test]
    fn test_atoms() {
        assert_eq!(parse_one("true").unwrap(),  Form::Bool(true));
        assert_eq!(parse_one("false").unwrap(), Form::Bool(false));
        assert_eq!(parse_one("nil").unwrap(),   Form::Nil);
        assert_eq!(parse_one("42").unwrap(),    Form::Int(42));
        assert_eq!(parse_one("-3").unwrap(),    Form::Int(-3));
        assert_eq!(parse_one("3.14").unwrap(),  Form::Float(3.14));
        assert_eq!(parse_one("x").unwrap(),     Form::Symbol("x".into()));
        assert_eq!(parse_one("mat-mul").unwrap(), Form::Symbol("mat-mul".into()));
    }
 
    #[test]
    fn test_string_literal() {
        assert_eq!(parse_one(r#""hello""#).unwrap(), Form::Str("hello".into()));
        assert_eq!(parse_one(r#""a\nb""#).unwrap(),  Form::Str("a\nb".into()));
    }
 
    #[test]
    fn test_simple_list() {
        let form = parse_one("(+ 1 2)").unwrap();
        assert_eq!(form, Form::List(vec![
            Form::Symbol("+".into()),
            Form::Int(1),
            Form::Int(2),
        ]));
    }
 
    #[test]
    fn test_nested_list() {
        let form = parse_one("(let [x 1] (+ x 2))").unwrap();
        assert_eq!(form, Form::List(vec![
            Form::Symbol("let".into()),
            Form::List(vec![Form::Symbol("x".into()), Form::Int(1)]),
            Form::List(vec![
                Form::Symbol("+".into()),
                Form::Symbol("x".into()),
                Form::Int(2),
            ]),
        ]));
    }
 
    #[test]
    fn test_square_brackets_same_as_parens() {
        assert_eq!(parse_one("[1 2 3]").unwrap(), parse_one("(1 2 3)").unwrap());
    }
 
    #[test]
    fn test_comment_ignored() {
        let form = parse_one("; esto es un comentario\n42").unwrap();
        assert_eq!(form, Form::Int(42));
    }
 
    #[test]
    fn test_comma_is_whitespace() {
        assert_eq!(parse_one("(1,2,3)").unwrap(), parse_one("(1 2 3)").unwrap());
    }
 
    #[test]
    fn test_multiple_top_level_forms() {
        let forms = parse("1 2 3").unwrap();
        assert_eq!(forms, vec![Form::Int(1), Form::Int(2), Form::Int(3)]);
    }
 
    #[test]
    fn test_parse_one_errors_on_multiple() {
        assert!(parse_one("1 2").is_err());
    }
 
    #[test]
    fn test_missing_close_paren() {
        let result = parse_one("(+ 1 2");
        println!("Result: {:?}", result);
        assert!(result.is_err());
    }
 
    #[test]
    fn test_unexpected_close_paren() {
        assert!(parse_one(")").is_err());
    }
 
    #[test]
    fn test_unterminated_string() {
        assert!(parse_one(r#""hello"#).is_err());
    }
 
    #[test]
    fn test_to_string_roundtrip() {
        let src = "(let [x 1] (+ x 2))";
        let form = parse_one(src).unwrap();
        // to_string produce paréntesis para todo (los [] se normalizan a ())
        assert_eq!(to_string(&form), "(let (x 1) (+ x 2))");
    }
 
    #[test]
    fn test_sample_program() {
        // Fragmento típico de FOPPL
        let src = r#"
            ; simple model
            (defn model []
              (let [mu (sample (normal 0.0 1.0))
                    sigma 1.0]
                (observe (normal mu sigma) 2.5)
                mu))
        "#;
        let forms = parse(src).unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0], Form::List(_)));
    }
}
