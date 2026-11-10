//! The non-interactive script runner and the agent-facing wire protocol.
//!
//! The contract this binary keeps is the one an external harness already depends on: one JSON
//! document on stdout and nothing else, a stable field set, and three exit codes
//! (0 success, 1 script or diagnostic failure, 2 usage). The --text flag selects human output.
//!
//! Positions published in the envelope are UTF-16 code-unit offsets into the caller's text,
//! which is what the C# runner publishes and what a consumer slices with.

use std::io::{Read, Write};

use lovelace_json::Json;
use lovelace_lang::Engine;

const PROTOCOL_VERSION: &str = "1";
const SYMBOLIC_FORMAT_VERSION: &str = "#!lovelace-sym 1";
const MATH_IR_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Json,
    Text,
}

struct Options {
    source: Option<String>,
    plot_dir: Option<String>,
    mode: Mode,
    omit_functions: bool,
    omit_variables: bool,
    precision: Option<usize>,
}

fn usage() -> String {
    let mut s = String::new();
    s.push_str("lovelace-run - evaluate a Lovelace script\n\n");
    s.push_str("USAGE:\n  lovelace-run (--eval <src> | --file <path> | --stdin | <path>) [options]\n\n");
    s.push_str("OPTIONS:\n");
    s.push_str("  --eval <src>        evaluate the given source\n");
    s.push_str("  --file <path>       evaluate a file\n");
    s.push_str("  --stdin             read the script from standard input\n");
    s.push_str("  --plot-dir <dir>    directory for plot output\n");
    s.push_str("  --precision <n>     display precision in decimal places (default 1000)\n");
    s.push_str("  --omit-functions    publish an empty functions array\n");
    s.push_str("  --omit-variables    publish an empty variables array\n");
    s.push_str("  --json              machine mode (default)\n");
    s.push_str("  --text              human mode\n");
    s.push_str("  --help, -h          this message\n");
    s
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        source: None,
        plot_dir: None,
        mode: Mode::Json,
        omit_functions: false,
        omit_variables: false,
        precision: None,
    };
    let mut bare: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].clone();
        let need = |i: usize| -> Result<String, String> {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{} requires a value", args[i]))
        };
        match a.as_str() {
            "--eval" => {
                o.source = Some(need(i)?);
                i += 2;
            }
            "--file" => {
                let path = need(i)?;
                let text =
                    std::fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
                o.source = Some(text);
                i += 2;
            }
            "--stdin" => {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .map_err(|e| format!("cannot read stdin: {e}"))?;
                // A shell may hand us a byte-order mark on stdin; it is transport, not source.
                if let Some(stripped) = buf.strip_prefix('\u{feff}') {
                    buf = stripped.to_string();
                }
                o.source = Some(buf);
                i += 1;
            }
            "--plot-dir" => {
                o.plot_dir = Some(need(i)?);
                i += 2;
            }
            "--precision" => {
                let v = need(i)?;
                o.precision =
                    Some(v.parse::<usize>().map_err(|_| format!("invalid precision '{v}'"))?);
                i += 2;
            }
            "--omit-functions" => {
                o.omit_functions = true;
                i += 1;
            }
            "--omit-variables" => {
                o.omit_variables = true;
                i += 1;
            }
            "--json" => {
                o.mode = Mode::Json;
                i += 1;
            }
            "--text" => {
                o.mode = Mode::Text;
                i += 1;
            }
            "--help" | "-h" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            other if other.starts_with('-') => return Err(format!("unknown option '{other}'")),
            other => {
                if bare.is_some() {
                    return Err("more than one script path given".to_string());
                }
                bare = Some(other.to_string());
                i += 1;
            }
        }
    }
    if o.source.is_none() {
        if let Some(p) = bare {
            let text = std::fs::read_to_string(&p).map_err(|e| format!("cannot read {p}: {e}"))?;
            o.source = Some(text);
        }
    } else if bare.is_some() {
        return Err("a script path was given together with another source".to_string());
    }
    if o.source.is_none() {
        return Err("no script given (use --eval, --file, --stdin, or a path)".to_string());
    }
    Ok(o)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("error: {msg}\n\n{}", usage());
            std::process::exit(2);
        }
    };

    let mut engine = Engine::new();
    if let Some(p) = opts.precision {
        engine.set_precision(p);
    }
    if let Some(dir) = &opts.plot_dir {
        engine.set_plot_dir(dir);
    }
    let source = opts.source.clone().unwrap_or_default();
    let started = std::time::Instant::now();
    let outcome = engine.eval(&source);
    let elapsed = started.elapsed();

    match (opts.mode, outcome) {
        (Mode::Json, Ok(value)) => {
            let env = success_envelope(&mut engine, &value, &opts, elapsed);
            print_json(&env);
        }
        (Mode::Json, Err(err)) => {
            let env = failure_envelope(&mut engine, &err, &source, elapsed);
            print_json(&env);
            std::process::exit(1);
        }
        (Mode::Text, Ok(value)) => {
            println!("= {}", engine.format_typed(&value));
            for line in engine.take_output() {
                println!("{line}");
            }
        }
        (Mode::Text, Err(err)) => {
            eprintln!("Error [EvaluationError/DomainError]: {}", err.message);
            std::process::exit(1);
        }
    }
}

fn print_json(env: &Json) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{}", env.to_string());
    let _ = out.flush();
}

fn preamble(ok: bool) -> Vec<(&'static str, Json)> {
    vec![
        ("protocolVersion", Json::Num(PROTOCOL_VERSION.to_string())),
        ("symbolicFormatVersion", Json::Str(SYMBOLIC_FORMAT_VERSION.to_string())),
        ("mathIrVersion", Json::Num(MATH_IR_VERSION.to_string())),
        ("ok", Json::Bool(ok)),
    ]
}

fn variable_json(name: String, typed: String, engine: &Engine, display_only: bool) -> Json {
    let _ = engine;
    let _ = display_only;
    Json::obj(vec![("name", Json::Str(name)), ("typed", Json::Str(typed))])
}

fn success_envelope(
    engine: &mut Engine,
    value: &lovelace_lang::Value,
    opts: &Options,
    elapsed: std::time::Duration,
) -> Json {
    let mut fields = preamble(true);
    fields.push(("revision", Json::Num(engine.revision().to_string())));
    let typed = engine.format_typed(value);
    let display = engine.format(value);
    let kind = value.kind_name().to_string();
    fields.push((
        "result",
        Json::obj(vec![
            ("kind", Json::Str(kind.clone())),
            ("display", Json::Str(display.clone())),
            ("typed", Json::Str(typed)),
            ("structured", Json::obj(vec![
                ("kind", Json::Str(kind)),
                ("display", Json::Str(display)),
            ])),
        ]),
    ));
    let output = engine.take_output();
    fields.push(("output", Json::str_array(output)));
    fields.push((
        "variables",
        if opts.omit_variables {
            Json::Arr(Vec::new())
        } else {
            let vars = engine.variables();
            Json::Arr(
                vars.into_iter()
                    .map(|(name, typed)| variable_json(name, typed, engine, false))
                    .collect(),
            )
        },
    ));
    fields.push((
        "functions",
        if opts.omit_functions {
            Json::Arr(Vec::new())
        } else {
            let fns = engine.functions();
            Json::Arr(
                fns.into_iter()
                    .map(|(name, params)| {
                        Json::obj(vec![
                            ("name", Json::Str(name)),
                            ("parameters", Json::str_array(params)),
                            ("builtin", Json::Bool(true)),
                        ])
                    })
                    .collect(),
            )
        },
    ));
    if let Some(p) = engine.last_plot() {
        fields.push((
            "plot",
            Json::obj(vec![
                ("path", Json::Str(p.path.clone())),
                ("title", Json::Str(p.title.clone())),
                ("svg", Json::Str(p.svg.clone())),
            ]),
        ));
    }
    fields.push(("elapsed", Json::Str(format_millis(elapsed))));
    fields.push(("artifacts", Json::Arr(Vec::new())));
    Json::obj(fields)
}

fn failure_envelope(
    engine: &mut Engine,
    err: &lovelace_lang::LangError,
    source: &str,
    elapsed: std::time::Duration,
) -> Json {
    let mut fields = preamble(false);
    fields.push(("revision", Json::Num(engine.revision().to_string())));
    fields.push(("code", Json::Str("EvaluationError".to_string())));
    fields.push(("category", Json::Str("DomainError".to_string())));
    fields.push(("message", Json::Str(err.message.clone())));
    fields.push(("recoverable", Json::Bool(true)));
    let (line, column) = line_col(source, err.position);
    fields.push((
        "diagnostics",
        Json::Arr(vec![Json::obj(vec![
            ("message", Json::Str(err.message.clone())),
            ("position", Json::Num(err.position.to_string())),
            ("line", Json::Num(line.to_string())),
            ("column", Json::Num(column.to_string())),
        ])]),
    ));
    fields.push(("elapsed", Json::Str(format_millis(elapsed))));
    let output = engine.take_output();
    fields.push(("output", Json::str_array(output)));
    fields.push(("artifacts", Json::Arr(Vec::new())));
    Json::obj(fields)
}

/// Converts a UTF-16 code-unit offset into a 1-based line and column.
fn line_col(source: &str, position: usize) -> (usize, usize) {
    let units: Vec<u16> = source.encode_utf16().collect();
    let upto = units.len().min(position);
    let mut line = 1usize;
    let mut col = 1usize;
    let mut i = 0usize;
    while i < upto {
        let u = units[i];
        if u == 13u16 {
            if i + 1 < upto && units[i + 1] == 10u16 {
                i += 1;
            }
            line += 1;
            col = 1;
        } else if u == 10u16 {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }
    (line, col)
}

fn format_millis(d: std::time::Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    format!("{:.2} ms", ms)
}
