use std::{env, fs};

use codemap::CodeMap;
use codemap_diagnostic::{ColorConfig, Diagnostic, Emitter, Level, SpanLabel, SpanStyle};
use rust_sitter::errors::{ParseError, ParseErrorReason};

mod parser;

fn convert_parse_error_to_diagnostics(
    file_span: &codemap::Span,
    error: &ParseError,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &error.reason {
        ParseErrorReason::MissingToken(tok) => diagnostics.push(Diagnostic {
            level: Level::Error,
            message: format!("Missing token: \"{tok}\""),
            code: Some("S000".to_string()),
            spans: vec![SpanLabel {
                span: file_span.subspan(error.start as u64, error.end as u64),
                style: SpanStyle::Primary,
                label: Some(format!("missing \"{tok}\"")),
            }],
        }),

        ParseErrorReason::UnexpectedToken(tok) => diagnostics.push(Diagnostic {
            level: Level::Error,
            message: format!("Unexpected token: \"{tok}\""),
            code: Some("S000".to_string()),
            spans: vec![SpanLabel {
                span: file_span.subspan(error.start as u64, error.end as u64),
                style: SpanStyle::Primary,
                label: Some(format!("unexpected \"{tok}\"")),
            }],
        }),

        ParseErrorReason::FailedNode(errors) => {
            if errors.is_empty() {
                diagnostics.push(Diagnostic {
                    level: Level::Error,
                    message: "Failed to parse node".to_string(),
                    code: Some("S000".to_string()),
                    spans: vec![SpanLabel {
                        span: file_span.subspan(error.start as u64, error.end as u64),
                        style: SpanStyle::Primary,
                        label: Some("failed".to_string()),
                    }],
                })
            } else {
                for error in errors {
                    convert_parse_error_to_diagnostics(file_span, error, diagnostics);
                }
            }
        }
    }
}

fn main() {
    let file = env::args().nth(1).expect("Expected file argument");
    let src = fs::read_to_string(&file).expect("Failed to read file");

    match parser::grammar::parse(src.as_str()) {
        Ok(json) => {
            #[cfg(debug_assertions)]
            {
                println!("{:#?}", json);
            }
            #[cfg(not(debug_assertions))]
            {
                std::hint::black_box(json);
            }
        }
        Err(errs) => {
            let mut codemap = CodeMap::new();
            let file_span = codemap.add_file(file, src.to_string());
            let mut diagnostics = vec![];
            for error in errs {
                convert_parse_error_to_diagnostics(&file_span.span, &error, &mut diagnostics);
            }

            let mut emitter = Emitter::stderr(ColorConfig::Always, Some(&codemap));
            emitter.emit(&diagnostics);
            std::process::exit(1);
        }
    }
}
