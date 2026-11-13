//! lovelace-know: the MGIR observation-driven behavioural graph CLI.
//!
//! JSON over stdio, the Lovelace.Run pattern: read a request on stdin (or --eval / --file),
//! run the command, write a response on stdout. Exit codes: 0 success, 1 command or
//! execution error, 2 usage error.

use std::io::{Read, Write};

use lovelace_know::cli;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&arguments));
}

fn run(arguments: &[String]) -> i32 {
    let mut eval: Option<String> = None;
    let mut file: Option<String> = None;
    let mut stdin_mode = false;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--eval" => {
                index += 1;
                if index >= arguments.len() {
                    return usage("--eval requires a JSON request argument.");
                }
                eval = Some(arguments[index].clone());
            }
            "--file" => {
                index += 1;
                if index >= arguments.len() {
                    return usage("--file requires a path argument.");
                }
                file = Some(arguments[index].clone());
            }
            "--stdin" => stdin_mode = true,
            "--help" | "-h" => {
                print_usage(&mut std::io::stdout());
                return 0;
            }
            other => return usage(&format!("Unknown argument '{other}'.")),
        }
        index += 1;
    }

    let request_json = if let Some(json) = eval {
        json
    } else if let Some(path) = file {
        match std::fs::read_to_string(&path) {
            // .NET's File.ReadAllText strips a UTF-8 BOM before parsing; match it.
            Ok(text) => text
                .strip_prefix('\u{feff}')
                .map(str::to_string)
                .unwrap_or(text),
            Err(error) => {
                let response =
                    cli::CliResponse::error("", format!("cannot read request file: {error}"));
                let _ = std::io::stdout().write_all(response.render().as_bytes());
                return 1;
            }
        }
    } else if stdin_mode {
        let mut text = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut text) {
            let response = cli::CliResponse::error("", format!("cannot read stdin: {error}"));
            let _ = std::io::stdout().write_all(response.render().as_bytes());
            return 1;
        }
        text
    } else {
        return usage("No request provided. Use --eval <json>, --file <path>, or --stdin.");
    };

    let (response, exit_code) = cli::run_request(&request_json);
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(response.as_bytes());
    let _ = stdout.flush();
    exit_code
}

fn usage(message: &str) -> i32 {
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "Error: {message}");
    print_usage(&mut stderr);
    2
}

fn print_usage<W: Write>(writer: &mut W) {
    let _ = writeln!(
        writer,
        "lovelace-know — observation-driven behavioural graph discovery CLI."
    );
    let _ = writeln!(writer);
    let _ = writeln!(writer, "Usage:");
    let _ = writeln!(writer, "  lovelace-know --eval <json request>");
    let _ = writeln!(writer, "  lovelace-know --file <request.json>");
    let _ = writeln!(writer, "  lovelace-know --stdin");
    let _ = writeln!(writer);
    let _ = writeln!(
        writer,
        "Commands: config | sample | reduce | converge | query"
    );
    let _ = writeln!(writer);
    let _ = writeln!(
        writer,
        "Request fields: command, config, graphPath, runner, seed, batchSize, maxSamples, query"
    );
    let _ = writeln!(writer, "  --help, -h   show this help");
}
