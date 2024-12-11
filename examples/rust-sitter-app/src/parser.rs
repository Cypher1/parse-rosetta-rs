use core::str;

#[derive(Debug)]
enum EscapeError {
    UnfinishedEscapeSequence,
    UnicodeError, // (Vec<u16>),
}

fn unescape(s: &str) -> Result<String, EscapeError> {
    let mut t: Vec<u16> = vec![];
    let mut unicode = false;
    let mut encoded: u16 = 0;
    let mut escape = 0; // The number of characters to escape.
    for i in s.chars() {
        if escape > 0 {
            let ch = match i {
                'n' => '\n' as u16,
                'b' => 8,
                'f' => 12,
                'r' => '\r' as u16,
                't' => '\t' as u16,
                '\'' => '\'' as u16,
                '"' => '"' as u16,
                '\\' => '\\' as u16,
                '/' => '/' as u16,
                'u' => {
                    escape = 4;
                    unicode = true;
                    encoded = 0;
                    continue;
                }
                '0'..'9' if unicode => (i as u16)-('0' as u16),
                'A'..'F' if unicode => 10+(i as u16)-('A' as u16),
                _ => panic!("CH >> {i:?}"),
            };
            escape -= 1;
            if !unicode {
                t.push(ch);
                continue;
            }
            // Handle the byte as hex encoded unicode.
            // TODO: This isn't correct in cases
            encoded *= 16;
            encoded += ch;
            if escape == 0 {
                t.push(encoded);
                unicode = false;
            }
        } else if i == '\\' {
            escape = 1;
        } else {
            t.push(i as u16);
        }
    }
    if escape > 0 {
        return Err(EscapeError::UnfinishedEscapeSequence);
    }
    match String::from_utf16(&t) {
        Ok(s) => Ok(s),
        Err(_) => Err(EscapeError::UnicodeError), // (t)),
    }
}

#[rust_sitter::grammar("parser")]
pub mod grammar {

    #[rust_sitter::language]
    #[derive(PartialEq, Eq, Debug)]
    pub enum JsonValue {
        #[rust_sitter::leaf(text = "null")]
        Null,
        #[rust_sitter::leaf(pattern = "true")]
        True,
        #[rust_sitter::leaf(pattern = "false")]
        False,
        Number(JsonNumber),
        Str(JsonString),
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
    pub struct JsonString(
        #[rust_sitter::leaf(pattern = r#""([^\"]|\\")*""#, transform = |v| crate::parser::unescape(&v[1..v.len()-1]).expect("?"))]
        pub String,
    );

    #[derive(PartialEq, Eq, Debug)]
    pub struct Property {
        name: JsonString,
        #[rust_sitter::leaf(text = r":")]
        sep: (),
        value: JsonValue,
    }
    impl Property {
        #[cfg(test)]
        pub fn new<S: Into<String>>(name: S, value: JsonValue) -> Self {
            Self {
                name: JsonString(name.into()),
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
    use super::grammar::{
        JsonNumber, JsonString, JsonValue, JsonValue::False, JsonValue::Null, JsonValue::True,
        Property,
    };
    #[allow(clippy::useless_attribute)]
    #[allow(dead_code)] // its dead for benches
    use super::*;
    use rust_sitter::errors::ParseError;

    fn jstr<S: Into<String>>(s: S) -> JsonValue {
        JsonValue::Str(JsonString(s.into()))
    }

    fn jobject(v: Vec<Property>) -> JsonValue {
        JsonValue::Object((), v, ())
    }

    fn jarray(v: Vec<JsonValue>) -> JsonValue {
        JsonValue::Array((), v, ())
    }

    fn jnum(f: f64) -> JsonValue {
        JsonValue::Number(JsonNumber::new(f))
    }

    #[allow(clippy::useless_attribute)]
    #[allow(dead_code)] // its dead for benches
    type Error = Vec<ParseError>;

    #[test]
    fn json_string() -> Result<(), Error> {
        assert_eq!(grammar::parse("\"\"")?, jstr(""));
        assert_eq!(grammar::parse("\"abc\"")?, jstr("abc"));
        assert_eq!(
            grammar::parse("\"abc\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0001\\u2014\u{2014}def\"")?,
            jstr("abc\"\\/\x08\x0C\n\r\t\x01——def"),
        );
        assert_eq!(grammar::parse("\"\\uD83D\\uDE10\"")?, jstr("😐"));

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
        let input = "{\"a\":42,\"b\":\"x\"}";

        let expected: JsonValue = jobject(vec![
            Property::new("a", jnum(42.0)),
            Property::new("b", jstr("x")),
        ]);

        assert_eq!(grammar::parse(input)?, expected);
        Ok(())
    }

    #[test]
    fn json_array() -> Result<(), Error> {
        let input = r#"[42,"x"]"#;

        let expected = jarray(vec![jnum(42.0), jstr("x")]);

        assert_eq!(grammar::parse(input)?, expected);
        Ok(())
    }

    #[test]
    fn json_whitespace() -> Result<(), Error> {
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
            jobject(vec![
                Property::new("null", Null),
                Property::new("true", True),
                Property::new("false", False),
                Property::new("number", jnum(123e4)),
                Property::new("string", jstr(" abc 123 ")),
                Property::new("array", jarray(vec![False, jnum(1.0), jstr("two")])),
                Property::new(
                    "object",
                    jobject(vec![
                        Property::new("a", jnum(1.0)),
                        Property::new("b", jstr("c")),
                    ])
                ),
                Property::new("empty_array", jarray(vec![]),),
                Property::new("empty_object", jobject(vec![]),),
            ])
        );
        Ok(())
    }
}
