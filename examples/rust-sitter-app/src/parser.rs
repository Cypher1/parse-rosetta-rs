#[rust_sitter::grammar("parser")]
pub mod grammar {

    fn unescape(s: &str) -> String {
        let mut t = String::new();
        let mut escape = false;
        for i in s.chars() {
            let ch = match i {
                '\\' if !escape => {
                    escape = true;
                    continue;
                }
                i if !escape => i,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\'' => '\'',
                '"' => '\"',
                '\\' => '\\',
                'u' => '\u{0}', //TODO
                '/' => '\\',
                _ => i, // TODO
            };
            t.push(ch);
        }
        t
    }

    #[rust_sitter::language]
    #[derive(PartialEq, Eq, Debug)]
    pub enum JsonValue {
        #[rust_sitter::leaf(text = "undefined")]
        Undefined,
        #[rust_sitter::leaf(text = "null")]
        Null,
        Str(
            #[rust_sitter::leaf(pattern = r#""([^\"]|\\")*""#, transform = |v| unescape(&v[1..v.len()-1]))]
             String,
        ),
        Boolean(#[rust_sitter::leaf(pattern = "(true|false)", transform = |v| v == "true")] bool),
        Number(JsonNumber),
        #[rust_sitter::delimited(
            #[rust_sitter::leaf(text = ",")]
            ()
        )]
        Array(
            #[rust_sitter::leaf(text = r"[")] (),
            #[rust_sitter::delimited(
                #[rust_sitter::leaf(text = ",")]
                ()
            )]
            Vec<JsonValue>,
            #[rust_sitter::leaf(text = r"]")] (),
        ),
        Object(
            #[rust_sitter::leaf(text = r"{")] (),
            #[rust_sitter::delimited(
                #[rust_sitter::leaf(text = ",")]
                ()
            )]
            Vec<Property>,
            #[rust_sitter::leaf(text = r"}")] (),
        ),
    }

    #[derive(PartialEq, Eq, Debug)]
    pub struct Property {
        #[rust_sitter::leaf(pattern = r#""([^\"]|\\")*""#, transform = |v| unescape(&v[1..v.len()-1]))]
        name: String,
        #[rust_sitter::leaf(text = r":")]
        sep: (),
        value: JsonValue,
    }
    impl Property {
        #[cfg(test)]
        pub fn new(name: String, value: JsonValue) -> Self {
            Self {
                name,
                sep: (),
                value,
            }
        }
    }

    #[derive(Debug)]
    pub struct JsonNumber {
        #[rust_sitter::leaf(pattern = r"\d+\.?\d*[eE]?\d*", transform = |v| v.parse().unwrap())]
        value: f64,
    }
    impl JsonNumber {
        #[cfg(test)]
        pub fn new(value: f64) -> Self {
            Self { value }
        }
    }

    impl PartialEq for JsonNumber {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }
    impl Eq for JsonNumber {}

    #[rust_sitter::extra]
    struct Whitespace {
        #[rust_sitter::leaf(pattern = r"\s")]
        _whitespace: (),
    }
}

#[cfg(test)]
mod test {
    use super::grammar::{JsonNumber, JsonValue, Property};
    #[allow(clippy::useless_attribute)]
    #[allow(dead_code)] // its dead for benches
    use super::*;
    use rust_sitter::errors::ParseError;
    use JsonValue::*;

    #[allow(clippy::useless_attribute)]
    #[allow(dead_code)] // its dead for benches
    type Error = Vec<ParseError>;

    #[test]
    fn json_string() -> Result<(), Error> {
        assert_eq!(grammar::parse("\"\"")?, Str("".to_string()));
        assert_eq!(grammar::parse("\"abc\"")?, Str("abc".to_string()));
        assert_eq!(
            grammar::parse("\"abc\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0001\\u2014\u{2014}def\"")?,
            Str("abc\"\\/\x08\x0C\n\r\t\x01——def".to_string()),
        );
        assert_eq!(grammar::parse("\"\\uD83D\\uDE10\"")?, Str("😐".to_string()));

        assert!(grammar::parse("\"").is_err());
        assert!(grammar::parse("\"abc").is_err());
        assert!(grammar::parse("\"\\\"").is_err());
        assert!(grammar::parse("\"\\u123\"").is_err());
        assert!(grammar::parse("\"\\uD800\"").is_err());
        assert!(grammar::parse("\"\\uD800\\uD800\"").is_err());
        assert!(grammar::parse("\"\\uDC00\"").is_err());

        Ok(())
    }

    #[test]
    fn json_object() -> Result<(), Error> {
        use JsonValue::{Number, Object, Str};

        let input = "{\"a\":42,\"b\":\"x\"}";

        let expected: JsonValue = Object(
            (),
            vec![
                Property::new("a".to_string(), Number(JsonNumber::new(42.0))),
                Property::new("b".to_string(), Str("x".to_string())),
            ],
            (),
        );

        assert_eq!(grammar::parse(input)?, expected);
        Ok(())
    }

    #[test]
    fn json_array() -> Result<(), Error> {
        use JsonValue::{Array, Number, Str};

        let input = r#"[42,"x"]"#;

        let expected = Array(
            (),
            vec![Number(JsonNumber::new(42.0)), Str("x".to_string())],
            (),
        );

        assert_eq!(grammar::parse(input)?, expected);
        Ok(())
    }

    #[test]
    fn json_whitespace() -> Result<(), Error> {
        use JsonValue::{Array, Boolean, Null, Number, Object, Str};

        let input = r#"
  {
    "null" : null,
    "true"  :true ,
    "false":  false  ,
    "number" : 123e4 ,
    "string" : " abc 123 " ,
    "array" : [ false , 1 , "two" ] ,
    "object" : { "a" : 1.0 , "b" : "c" } ,
    "empty_array" : [  ] ,
    "empty_object" : {   }
  }
  "#;

        assert_eq!(
            grammar::parse(input)?,
            Object(
                (),
                vec![
                    ("null".to_string(), Null),
                    ("true".to_string(), Boolean(true)),
                    ("false".to_string(), Boolean(false)),
                    ("number".to_string(), Number(JsonNumber::new(123e4))),
                    ("string".to_string(), Str(" abc 123 ".to_string())),
                    (
                        "array".to_string(),
                        Array(
                            (),
                            vec![
                                Boolean(false),
                                Number(JsonNumber::new(1.0)),
                                Str("two".to_string())
                            ],
                            ()
                        )
                    ),
                    (
                        "object".to_string(),
                        Object(
                            (),
                            vec![
                                ("a".to_string(), Number(JsonNumber::new(1.0))),
                                ("b".to_string(), Str("c".to_string())),
                            ]
                            .into_iter()
                            .map(|(x, y)| Property::new(x, y))
                            .collect(),
                            ()
                        )
                    ),
                    ("empty_array".to_string(), Array((), vec![], ()),),
                    ("empty_object".to_string(), Object((), Vec::new(), ()),),
                ]
                .into_iter()
                .map(|(x, y)| Property::new(x, y))
                .collect(),
                ()
            )
        );
        Ok(())
    }
}
