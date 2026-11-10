//! The parity gate: runs every example in `parity/corpus/*.json` through the engine and
//! compares the observed rendering with the documented one.
//!
//! Exit code 0 means every example matched; 1 means at least one diverged; 2 means the corpus
//! could not be loaded. The gate is deliberately loud: a missing corpus file is a failure, never
//! an empty pass.

use std::process::ExitCode;

use lovelace_json::Json;
use lovelace_lang::Engine;

struct Example {
    id: String,
    script: String,
    kind: String,
    text: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filter = args.iter().find(|a| !a.starts_with("--")).cloned();
    let verbose = args.iter().any(|a| a == "--verbose");

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let files = [
        root.join("parity").join("corpus").join("lang.json"),
        root.join("parity").join("corpus").join("symbolics.json"),
    ];

    let mut examples = Vec::new();
    for path in files {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("cannot read corpus {}: {e}", path.display());
                return ExitCode::from(2);
            }
        };
        let doc = match Json::parse(&text) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("cannot parse corpus {}: {e}", path.display());
                return ExitCode::from(2);
            }
        };
        let Some(items) = doc.get("examples").and_then(|v| v.as_arr()) else {
            eprintln!("corpus {} has no examples array", path.display());
            return ExitCode::from(2);
        };
        for it in items {
            let field = |k: &str| it.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            examples.push(Example {
                id: field("id"),
                script: field("script"),
                kind: field("kind"),
                text: field("text"),
            });
        }
    }

    if examples.is_empty() {
        eprintln!("corpus is empty: refusing a vacuous pass");
        return ExitCode::from(2);
    }

    let total = examples.len();
    let mut passed = 0usize;
    let mut failures: Vec<(String, String, String, String)> = Vec::new();

    for ex in &examples {
        if let Some(f) = &filter {
            if !ex.id.starts_with(f.as_str()) {
                continue;
            }
        }
        let mut engine = Engine::new();
        engine.set_precision(1000);
        let observed = run_example(&mut engine, ex);
        match observed {
            Ok(got) if got == ex.text => {
                passed += 1;
                if verbose {
                    println!("PASS {} | {}", ex.id, one_line(&ex.script));
                }
            }
            Ok(got) => {
                failures.push((ex.id.clone(), one_line(&ex.script), ex.text.clone(), got));
            }
            Err(msg) => {
                failures.push((ex.id.clone(), one_line(&ex.script), ex.text.clone(), format!("<harness error: {msg}>")));
            }
        }
    }

    for (id, script, want, got) in &failures {
        println!("FAIL {id}\n  script:   {script}\n  expected: {want}\n  observed: {got}");
    }

    println!(
        "\ncorpus: {passed} passed, {} failed, {total} total{}",
        failures.len(),
        filter.map(|f| format!(" (filter {f})")).unwrap_or_default()
    );
    if failures.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(1) }
}

fn one_line(s: &str) -> String {
    s.replace('\n', " ; ")
}

fn run_example(engine: &mut Engine, ex: &Example) -> Result<String, String> {
    match ex.kind.as_str() {
        "value" => match engine.eval(&ex.script) {
            Ok(v) => Ok(engine.format_typed(&v)),
            Err(e) => Err(format!("error: {}", e.message)),
        },
        "error" => match engine.eval(&ex.script) {
            Ok(v) => Ok(format!("no error (got {})", engine.format_typed(&v))),
            Err(e) => Ok(e.message),
        },
        "prints" => match engine.eval(&ex.script) {
            Ok(_) => {
                let lines = engine.take_output();
                Ok(lines.join("\n"))
            }
            Err(e) => Err(format!("error: {}", e.message)),
        },
        "plot" => match engine.eval(&ex.script) {
            Ok(_) => match engine.last_plot() {
                Some(p) => Ok(p.title),
                None => Ok("<no plot captured>".to_string()),
            },
            Err(e) => Err(format!("error: {}", e.message)),
        },
        other => Err(format!("unknown expectation kind '{other}'")),
    }
}
