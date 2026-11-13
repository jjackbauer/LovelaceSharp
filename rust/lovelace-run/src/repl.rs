//! The interactive REPL over the existing engine.
//!
//! The transcript contract is the reference console's (Lovelace.Console/Repl/ReplSession.cs):
//! the same banner, `» ` / `… ` prompts, brace-balanced continuation accumulation, the same
//! command set and messages, results rendered with the engine's typed display plus a scaled
//! elapsed suffix, and errors rendered with a caret when the message names a position. Input is
//! read through a dependency-free line editor: plain lines when stdin/stdout are pipes (which is
//! what a transcript test drives), key-by-key editing and history on a terminal.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use lovelace_lang::{Engine, Value, help};

use crate::{Options, decode_script_bytes};

/// The reference console's banner version (`Program.cs`, assembly version 1.0.0).
const VERSION: &str = "1.0.0";
/// The statement prompt.
const PROMPT: &str = "» ";
/// The continuation prompt shown while braces are unbalanced.
const CONTINUATION_PROMPT: &str = "… ";

/// Runs an interactive session to end of input, then exits the process successfully.
pub fn run(opts: &Options) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    // The headless path is selected by redirected streams; a terminal gets the editor.
    let interactive = stdin.is_terminal() && stdout.is_terminal();
    let mut out = stdout.lock();

    let _ = writeln!(out, "LovelaceSharp REPL v{VERSION}");
    let _ = writeln!(
        out,
        "Arbitrary-precision math scripting, vector math, and plotting."
    );
    let _ = writeln!(
        out,
        "Type 'help' for a list of statements, operators, functions, and commands."
    );
    let _ = writeln!(out);

    let mut engine = Engine::new();
    if let Some(dir) = &opts.plot_dir {
        engine.set_plot_dir(dir);
    }
    if let Some(name) = &opts.plot_file {
        engine.set_plot_file(name);
    }
    // The runner's deadline is per evaluation; in a session it bounds each command.
    engine.set_cancel_after(opts.cancel_after.map(Duration::from_millis));

    let mut session = ReplSession::new(engine);
    let mut editor = LineEditor::new(interactive);
    let mut input = io::BufReader::new(stdin.lock());

    loop {
        let line = match editor.read_line(&mut input, &mut out, PROMPT) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Continuation lines are appended verbatim while braces are unbalanced, exactly as the
        // reference accumulates them (the FIRST line alone is trimmed).
        let mut source = trimmed.to_string();
        while !braces_balanced(&source) {
            match editor.read_line(&mut input, &mut out, CONTINUATION_PROMPT) {
                Ok(Some(more)) => {
                    source.push('\n');
                    source.push_str(&more);
                }
                Ok(None) => {
                    session.write_line(&mut out, "Bye!");
                    return;
                }
                Err(_) => {
                    session.write_line(&mut out, "Bye!");
                    return;
                }
            }
        }

        match session.handle_command(&mut out, &source) {
            CommandResult::Exit => break,
            CommandResult::Handled => continue,
            CommandResult::NotACommand => session.evaluate(&mut out, &source, &source),
        }
    }

    session.write_line(&mut out, "Bye!");
}

/// The engine plus the two pieces of session state the reference keeps beside it: the last
/// evaluation's elapsed time (special commands report the previous one) and the deadline.
struct ReplSession {
    engine: Engine,
    last_elapsed: Duration,
}

impl ReplSession {
    fn new(engine: Engine) -> ReplSession {
        ReplSession {
            engine,
            last_elapsed: Duration::ZERO,
        }
    }

    fn write_line<W: Write>(&self, out: &mut W, text: &str) {
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    }

    /// Evaluates one accumulated statement and renders its output, result, or failure.
    /// `error_source` is the text a caret is drawn under: the statement itself, except for
    /// `run <file>`, whose failure position belongs to a source the user cannot see echoed.
    fn evaluate<W: Write>(&mut self, out: &mut W, source: &str, error_source: &str) {
        let started = Instant::now();
        let outcome = self.engine.eval(source);
        let elapsed = started.elapsed();
        self.last_elapsed = elapsed;

        // print() output belongs to the same transcript and precedes the result line, exactly as
        // the reference's injected TextWriter receives it during evaluation.
        for line in self.engine.take_output() {
            self.write_line(out, &line);
        }

        match outcome {
            Ok(Value::Void) => {}
            Ok(value) => {
                let rendered = self.engine.format_typed(&value);
                self.write_line(
                    out,
                    &format!("= {rendered}   [{}]", format_elapsed(elapsed)),
                );
            }
            Err(error) => self.print_error(out, error_source, &error.message, elapsed),
        }
    }

    fn print_error<W: Write>(&self, out: &mut W, source: &str, message: &str, elapsed: Duration) {
        if let Some(position) = error_position(message) {
            self.write_line(out, source);
            self.write_line(out, &format!("{}^", " ".repeat(position)));
        }
        self.write_line(
            out,
            &format!("Error: {message}   [{}]", format_elapsed(elapsed)),
        );
    }

    /// Dispatches the reference's special commands. `NotACommand` means the text is Lovelace.
    fn handle_command<W: Write>(&mut self, out: &mut W, source: &str) -> CommandResult {
        if source == "exit" || source == "quit" {
            return CommandResult::Exit;
        }
        if source == "help" {
            self.write_line(out, &help::overview());
            return CommandResult::Handled;
        }
        if let Some(argument) = source.strip_prefix("help ") {
            // The reference renders a function, else a category; an unknown argument receives
            // the category renderer's own "No category ..." sentence.
            let argument = argument.trim();
            let text = help::function(argument).unwrap_or_else(|| help::category(argument));
            self.write_line(out, &text);
            return CommandResult::Handled;
        }
        if source == "vars" {
            self.print_vars(out);
            return CommandResult::Handled;
        }
        if source == "funcs" {
            self.write_line(out, &help::funcs(None));
            return CommandResult::Handled;
        }
        if let Some(category) = source.strip_prefix("funcs ") {
            self.write_line(out, &help::funcs(Some(category.trim())));
            return CommandResult::Handled;
        }
        if source == "set pretty unicode" {
            self.engine.set_unicode_output(true);
            self.write_line(out, "Pretty output: unicode.");
            return CommandResult::Handled;
        }
        if source == "set pretty ascii" {
            self.engine.set_unicode_output(false);
            self.write_line(out, "Pretty output: ascii.");
            return CommandResult::Handled;
        }
        if source == "clear" {
            let names: Vec<String> = self
                .engine
                .variable_values()
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            for name in names {
                self.engine.remove_variable(&name);
            }
            self.write_line(out, "All variables cleared.");
            return CommandResult::Handled;
        }
        if let Some(name) = source.strip_prefix("delete ") {
            let name = name.trim();
            if self.engine.remove_variable(name) {
                self.write_line(out, &format!("Variable '{name}' deleted."));
            } else {
                self.write_line(out, &format!("Variable '{name}' is not defined."));
            }
            return CommandResult::Handled;
        }
        if let Some(value) = source.strip_prefix("set precision ") {
            let value = value.trim();
            match positive_integer(value) {
                Some(decimals) => {
                    // The reference moves BOTH knobs; setting the display override too keeps a
                    // previous `set display` from surviving a later `set precision`.
                    self.engine.set_precision(decimals);
                    self.engine.set_display_precision(decimals);
                    self.write_line(
                        out,
                        &format!("Computation precision set to {decimals} decimal places."),
                    );
                }
                None => self.write_line(
                    out,
                    &format!("Invalid argument '{value}': expected a positive integer."),
                ),
            }
            return CommandResult::Handled;
        }
        if let Some(value) = source.strip_prefix("set display ") {
            let value = value.trim();
            match positive_integer(value) {
                Some(decimals) => {
                    // A display-only knob still marks precision explicit, as the reference's
                    // property setter does, but it must not move the computation bound.
                    let computation = self.engine.precision();
                    self.engine.set_precision(computation);
                    self.engine.set_display_precision(decimals);
                    self.write_line(out, &format!("Display digits set to {decimals}."));
                }
                None => self.write_line(
                    out,
                    &format!("Invalid argument '{value}': expected a positive integer."),
                ),
            }
            return CommandResult::Handled;
        }
        if let Some(path) = source.strip_prefix("run ") {
            let path = path.trim().trim_matches('"');
            match read_script(path) {
                Ok(text) => self.evaluate(out, &text, source),
                Err(message) => self.print_error(out, source, &message, self.last_elapsed),
            }
            return CommandResult::Handled;
        }
        CommandResult::NotACommand
    }

    fn print_vars<W: Write>(&self, out: &mut W) {
        let variables = self.engine.variable_values();
        if variables.is_empty() {
            self.write_line(out, "(no variables defined)");
            return;
        }
        for (name, value) in variables {
            let rendered = self.engine.format_typed(&value);
            self.write_line(out, &format!("  {name} = {rendered}"));
        }
    }
}

enum CommandResult {
    Exit,
    Handled,
    NotACommand,
}

/// Reads a script for `run`, decoding the same byte-order marks the file surface accepts and
/// dropping the mark as File.ReadAllText does. A read failure crosses as the reference's CLR
/// sentence, because that sentence is part of the transcript.
fn read_script(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("The value cannot be an empty string. (Parameter 'path')".to_string());
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let absolute = std::path::absolute(Path::new(path))
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string());
            return Err(format!("Could not find file '{absolute}'."));
        }
        Err(error) => return Err(error.to_string()),
    };
    let text = decode_script_bytes(&bytes)?;
    Ok(text
        .strip_prefix('\u{feff}')
        .map(str::to_string)
        .unwrap_or(text))
}

/// The reference's positive-integer parse for `set precision` / `set display`.
fn positive_integer(text: &str) -> Option<usize> {
    let value = text.parse::<i64>().ok()?;
    if value <= 0 {
        return None;
    }
    Some(value as usize)
}

/// The reference's `BracesBalanced`: it counts braces in the raw text (a brace inside a string
/// still counts) and treats a negative depth as balanced.
fn braces_balanced(text: &str) -> bool {
    let mut depth: i32 = 0;
    for character in text.chars() {
        if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth -= 1;
        }
    }
    depth <= 0
}

/// The first `at position <digits>` in a message, case-insensitively, as the reference's
/// regex extraction does.
fn error_position(message: &str) -> Option<usize> {
    const NEEDLE: &str = "at position ";
    let lower = message.to_ascii_lowercase();
    let start = lower.find(NEEDLE)? + NEEDLE.len();
    let digits: String = message[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// The reference's `Timing.Format`: the largest unit worth at least one whole unit, at most two
/// trimmed decimals, nanoseconds whole.
fn format_elapsed(elapsed: Duration) -> String {
    let total_nanoseconds = elapsed.as_nanos() as f64;
    if total_nanoseconds < 1_000.0 {
        return format!("{:.0} ns", total_nanoseconds);
    }
    let total_microseconds = total_nanoseconds / 1_000.0;
    if total_microseconds < 1_000.0 {
        return format!("{} µs", two_decimals(total_microseconds));
    }
    let total_milliseconds = total_nanoseconds / 1_000_000.0;
    if total_milliseconds < 1_000.0 {
        return format!("{} ms", two_decimals(total_milliseconds));
    }
    let total_seconds = total_nanoseconds / 1_000_000_000.0;
    if total_seconds < 60.0 {
        return format!("{} s", two_decimals(total_seconds));
    }
    let total_minutes = total_seconds / 60.0;
    if total_minutes < 60.0 {
        return format!("{} min", two_decimals(total_minutes));
    }
    format!("{} h", two_decimals(total_seconds / 3_600.0))
}

fn two_decimals(value: f64) -> String {
    let mut text = format!("{value:.2}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

// ---------------------------------------------------------------------------------------------
// Line input: plain when redirected, a key-by-key editor with history on a terminal.
// ---------------------------------------------------------------------------------------------

struct LineEditor {
    interactive: bool,
    history: Vec<String>,
}

impl LineEditor {
    fn new(interactive: bool) -> LineEditor {
        LineEditor {
            interactive,
            history: Vec::new(),
        }
    }

    fn remember(&mut self, line: &str) {
        if !line.trim().is_empty() {
            self.history.push(line.to_string());
        }
    }

    /// One line with its prompt, or `None` at end of input / Ctrl+C (both mean exit intent).
    fn read_line<R: BufRead, W: Write>(
        &mut self,
        input: &mut R,
        out: &mut W,
        prompt: &str,
    ) -> io::Result<Option<String>> {
        if self.interactive {
            if let Some(_guard) = terminal::RawMode::enable() {
                return self.read_interactive(input, out, prompt);
            }
        }
        self.read_plain(input, out, prompt)
    }

    /// The headless path: read with `read_line`, echo the line, and treat EOF as exit intent.
    fn read_plain<R: BufRead, W: Write>(
        &mut self,
        input: &mut R,
        out: &mut W,
        prompt: &str,
    ) -> io::Result<Option<String>> {
        write!(out, "{prompt}")?;
        out.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            writeln!(out)?;
            return Ok(None);
        }
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        writeln!(out, "{line}")?;
        out.flush()?;
        self.remember(&line);
        Ok(Some(line))
    }

    fn read_interactive<R: BufRead, W: Write>(
        &mut self,
        input: &mut R,
        out: &mut W,
        prompt: &str,
    ) -> io::Result<Option<String>> {
        write!(out, "{prompt}")?;
        out.flush()?;
        let mut buffer = LineBuffer::new(&self.history);
        loop {
            let Some(key) = read_key(input)? else {
                writeln!(out)?;
                return Ok(None);
            };
            match buffer.handle(key) {
                Step::Submit(line) => {
                    write!(out, "\r\n")?;
                    out.flush()?;
                    self.remember(&line);
                    return Ok(Some(line));
                }
                Step::Interrupt => {
                    write!(out, "\r\n")?;
                    out.flush()?;
                    return Ok(None);
                }
                Step::Redraw => redraw(out, prompt, &buffer)?,
                Step::Unchanged => {}
            }
        }
    }
}

fn redraw<W: Write>(out: &mut W, prompt: &str, buffer: &LineBuffer) -> io::Result<()> {
    let text: String = buffer.chars.iter().collect();
    write!(out, "\r\u{1b}[K{prompt}{text}")?;
    let back = buffer.chars.len().saturating_sub(buffer.cursor);
    if back > 0 {
        write!(out, "\u{1b}[{back}D")?;
    }
    out.flush()
}

/// One key the editor understands.
#[derive(Clone, Copy)]
enum Key {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    Interrupt,
    Ignore,
}

/// What a key did to the line.
enum Step {
    Submit(String),
    Interrupt,
    Redraw,
    Unchanged,
}

/// The pure editing state: buffer, cursor, history position and the draft saved before a history
/// recall. This is the whole behaviour CLI-003 asks for and is unit-tested without a terminal.
struct LineBuffer {
    chars: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    history_index: usize,
    pending: Option<String>,
}

impl LineBuffer {
    fn new(history: &[String]) -> LineBuffer {
        LineBuffer {
            chars: Vec::new(),
            cursor: 0,
            history: history.to_vec(),
            history_index: history.len(),
            pending: None,
        }
    }

    fn text(&self) -> String {
        self.chars.iter().collect()
    }

    fn replace(&mut self, text: &str) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
    }

    fn handle(&mut self, key: Key) -> Step {
        match key {
            Key::Interrupt => Step::Interrupt,
            Key::Enter => Step::Submit(self.text()),
            Key::Char(character) => {
                if character.is_control() {
                    return Step::Unchanged;
                }
                // Typing ends history navigation and discards the saved draft.
                self.history_index = self.history.len();
                self.pending = None;
                self.chars.insert(self.cursor, character);
                self.cursor += 1;
                Step::Redraw
            }
            Key::Backspace => {
                if self.cursor == 0 {
                    return Step::Unchanged;
                }
                self.cursor -= 1;
                self.chars.remove(self.cursor);
                Step::Redraw
            }
            Key::Delete => {
                if self.cursor >= self.chars.len() {
                    return Step::Unchanged;
                }
                self.chars.remove(self.cursor);
                Step::Redraw
            }
            Key::Left => {
                if self.cursor == 0 {
                    return Step::Unchanged;
                }
                self.cursor -= 1;
                Step::Redraw
            }
            Key::Right => {
                if self.cursor >= self.chars.len() {
                    return Step::Unchanged;
                }
                self.cursor += 1;
                Step::Redraw
            }
            Key::Home => {
                if self.cursor == 0 {
                    return Step::Unchanged;
                }
                self.cursor = 0;
                Step::Redraw
            }
            Key::End => {
                if self.cursor == self.chars.len() {
                    return Step::Unchanged;
                }
                self.cursor = self.chars.len();
                Step::Redraw
            }
            Key::Up => {
                if self.history.is_empty() {
                    return Step::Unchanged;
                }
                if self.history_index == self.history.len() {
                    self.pending = Some(self.text());
                }
                if self.history_index == 0 {
                    return Step::Unchanged;
                }
                self.history_index -= 1;
                let entry = self.history[self.history_index].clone();
                self.replace(&entry);
                Step::Redraw
            }
            Key::Down => {
                if self.history_index >= self.history.len() {
                    return Step::Unchanged;
                }
                self.history_index += 1;
                let replacement = if self.history_index == self.history.len() {
                    self.pending.take().unwrap_or_default()
                } else {
                    self.history[self.history_index].clone()
                };
                self.replace(&replacement);
                Step::Redraw
            }
            Key::Ignore => Step::Unchanged,
        }
    }
}

/// Decodes one key from the byte stream. On a terminal in raw mode the arrow/navigation keys
/// arrive as ANSI (or Windows virtual-terminal) sequences; Ctrl+C is byte 0x03 because processed
/// input is disabled.
fn read_key<R: BufRead>(input: &mut R) -> io::Result<Option<Key>> {
    let Some(byte) = read_byte(input)? else {
        return Ok(None);
    };
    let key = match byte {
        0x03 => Key::Interrupt,
        b'\r' | b'\n' => Key::Enter,
        0x7f | 0x08 => Key::Backspace,
        0x1b => read_escape(input)?,
        0x00..=0x1f => Key::Ignore,
        _ => Key::Char(read_character(input, byte)?),
    };
    Ok(Some(key))
}

fn read_byte<R: BufRead>(input: &mut R) -> io::Result<Option<u8>> {
    let mut byte = [0u8; 1];
    if input.read(&mut byte)? == 0 {
        Ok(None)
    } else {
        Ok(Some(byte[0]))
    }
}

fn read_escape<R: BufRead>(input: &mut R) -> io::Result<Key> {
    let Some(kind) = read_byte(input)? else {
        return Ok(Key::Ignore);
    };
    match kind {
        b'[' => {
            let Some(code) = read_byte(input)? else {
                return Ok(Key::Ignore);
            };
            let key = match code {
                b'A' => Key::Up,
                b'B' => Key::Down,
                b'C' => Key::Right,
                b'D' => Key::Left,
                b'H' => Key::Home,
                b'F' => Key::End,
                b'1' | b'7' => {
                    let _ = read_byte(input)?;
                    Key::Home
                }
                b'4' | b'8' => {
                    let _ = read_byte(input)?;
                    Key::End
                }
                b'3' => {
                    let _ = read_byte(input)?;
                    Key::Delete
                }
                _ => Key::Ignore,
            };
            Ok(key)
        }
        b'O' => match read_byte(input)? {
            Some(b'H') => Ok(Key::Home),
            Some(b'F') => Ok(Key::End),
            _ => Ok(Key::Ignore),
        },
        _ => Ok(Key::Ignore),
    }
}

fn read_character<R: BufRead>(input: &mut R, first: u8) -> io::Result<char> {
    if first < 0x80 {
        return Ok(first as char);
    }
    let length = if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else if first >> 3 == 0b11110 {
        4
    } else {
        return Ok('\u{fffd}');
    };
    let mut bytes = [0u8; 4];
    bytes[0] = first;
    for slot in bytes.iter_mut().take(length).skip(1) {
        let Some(byte) = read_byte(input)? else {
            return Ok('\u{fffd}');
        };
        *slot = byte;
    }
    Ok(std::str::from_utf8(&bytes[..length])
        .ok()
        .and_then(|text| text.chars().next())
        .unwrap_or('\u{fffd}'))
}

// ---------------------------------------------------------------------------------------------
// Terminal raw mode. Windows enables virtual-terminal input/output through kernel32; Unix uses
// stty. Both are dependency-free and restore the terminal when the editor leaves.
// ---------------------------------------------------------------------------------------------

#[cfg(windows)]
#[allow(unsafe_code)]
mod terminal {
    use std::ffi::c_void;

    const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> *mut c_void;
        fn GetConsoleMode(handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: *mut c_void, mode: u32) -> i32;
    }

    pub struct RawMode {
        input: *mut c_void,
        output: *mut c_void,
        input_mode: u32,
        output_mode: u32,
    }

    impl RawMode {
        pub fn enable() -> Option<RawMode> {
            let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
            let output = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
            let mut input_mode = 0u32;
            let mut output_mode = 0u32;
            if unsafe { GetConsoleMode(input, &mut input_mode) } == 0
                || unsafe { GetConsoleMode(output, &mut output_mode) } == 0
            {
                return None;
            }
            let raw_input = (input_mode
                & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            let raw_output = output_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if unsafe { SetConsoleMode(input, raw_input) } == 0 {
                return None;
            }
            if unsafe { SetConsoleMode(output, raw_output) } == 0 {
                unsafe { SetConsoleMode(input, input_mode) };
                return None;
            }
            Some(RawMode {
                input,
                output,
                input_mode,
                output_mode,
            })
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            unsafe {
                SetConsoleMode(self.input, self.input_mode);
                SetConsoleMode(self.output, self.output_mode);
            }
        }
    }
}

#[cfg(unix)]
mod terminal {
    use std::process::Command;

    pub struct RawMode {
        saved: String,
    }

    impl RawMode {
        pub fn enable() -> Option<RawMode> {
            let saved = Command::new("stty").arg("-g").output().ok()?;
            if !saved.status.success() {
                return None;
            }
            let saved = String::from_utf8_lossy(&saved.stdout).trim().to_string();
            let status = Command::new("stty").args(["raw", "-echo"]).status().ok()?;
            if !status.success() {
                return None;
            }
            Some(RawMode { saved })
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            let _ = Command::new("stty").arg(&self.saved).status();
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod terminal {
    pub struct RawMode;

    impl RawMode {
        pub fn enable() -> Option<RawMode> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> LineBuffer {
        let mut buffer = LineBuffer::new(&[]);
        for character in text.chars() {
            buffer.handle(Key::Char(character));
        }
        buffer
    }

    #[test]
    fn line_buffer_edits_at_the_cursor() {
        let mut buffer = typed("helo");
        buffer.handle(Key::Left);
        buffer.handle(Key::Left);
        buffer.handle(Key::Char('l'));
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.cursor, 3);

        buffer.handle(Key::Home);
        buffer.handle(Key::Delete);
        assert_eq!(buffer.text(), "ello");
        buffer.handle(Key::End);
        buffer.handle(Key::Backspace);
        assert_eq!(buffer.text(), "ell");
        buffer.handle(Key::Right);
        assert_eq!(buffer.cursor, 3);
    }

    #[test]
    fn history_navigation_restores_the_draft() {
        let history = vec!["a = 5".to_string(), "help".to_string()];
        let mut buffer = LineBuffer::new(&history);
        assert!(matches!(buffer.handle(Key::Up), Step::Redraw));
        assert_eq!(buffer.text(), "help");
        assert!(matches!(buffer.handle(Key::Up), Step::Redraw));
        assert_eq!(buffer.text(), "a = 5");
        assert!(matches!(buffer.handle(Key::Up), Step::Unchanged));
        assert!(matches!(buffer.handle(Key::Down), Step::Redraw));
        assert_eq!(buffer.text(), "help");
        assert!(matches!(buffer.handle(Key::Down), Step::Redraw));
        assert_eq!(buffer.text(), "");
        assert!(matches!(buffer.handle(Key::Down), Step::Unchanged));

        buffer.handle(Key::Up);
        buffer.handle(Key::Char('x'));
        assert_eq!(buffer.text(), "helpx");
    }

    #[test]
    fn ansi_navigation_sequences_decode_to_keys() {
        fn key(bytes: &[u8]) -> Option<Key> {
            read_key(&mut std::io::Cursor::new(bytes)).expect("the cursor cannot fail")
        }
        assert!(matches!(key(b"\x1b[A"), Some(Key::Up)));
        assert!(matches!(key(b"\x1b[B"), Some(Key::Down)));
        assert!(matches!(key(b"\x1b[C"), Some(Key::Right)));
        assert!(matches!(key(b"\x1b[D"), Some(Key::Left)));
        assert!(matches!(key(b"\x1b[H"), Some(Key::Home)));
        assert!(matches!(key(b"\x1b[F"), Some(Key::End)));
        assert!(matches!(key(b"\x1b[3~"), Some(Key::Delete)));
        assert!(matches!(key(b"\x1b[1~"), Some(Key::Home)));
        assert!(matches!(key(b"\x1b[4~"), Some(Key::End)));
        assert!(matches!(key(b"\x03"), Some(Key::Interrupt)));
        assert!(matches!(key(b"\r"), Some(Key::Enter)));
        assert!(matches!(key(b"\x7f"), Some(Key::Backspace)));
        assert!(matches!(key("é".as_bytes()), Some(Key::Char('é'))));
    }

    #[test]
    fn ctrl_c_interrupts_the_line() {
        let mut buffer = typed("1 + 1");
        assert!(matches!(buffer.handle(Key::Interrupt), Step::Interrupt));
    }

    #[test]
    fn braces_must_balance_for_continuation() {
        assert!(braces_balanced("a = 5"));
        assert!(braces_balanced("func f(x) { x + 1 }"));
        assert!(!braces_balanced("func f(x) {"));
        assert!(!braces_balanced("func f(x) {\n  x + 1"));
        // The reference counts braces in strings too; a bare '}' makes depth negative = balanced.
        assert!(braces_balanced("print(\"}\")"));
        assert!(braces_balanced("}"));
    }

    #[test]
    fn error_positions_are_extracted_case_insensitively() {
        assert_eq!(
            error_position("Expected 'RBracket' but found ';' at position 4."),
            Some(4)
        );
        assert_eq!(error_position("At Position 12, something"), Some(12));
        assert_eq!(error_position("Undefined variable 'x'."), None);
    }

    #[test]
    fn elapsed_uses_the_reference_units() {
        assert_eq!(format_elapsed(Duration::from_nanos(999)), "999 ns");
        assert_eq!(format_elapsed(Duration::from_nanos(1_500)), "1.5 µs");
        assert_eq!(format_elapsed(Duration::from_nanos(1_234_567)), "1.23 ms");
        assert_eq!(format_elapsed(Duration::from_millis(2_500)), "2.5 s");
        assert_eq!(format_elapsed(Duration::from_secs(90)), "1.5 min");
        assert_eq!(format_elapsed(Duration::from_secs(3_600)), "1 h");
    }

    #[test]
    fn positive_integer_accepts_the_reference_forms() {
        assert_eq!(positive_integer("5"), Some(5));
        assert_eq!(positive_integer("+5"), Some(5));
        assert_eq!(positive_integer("007"), Some(7));
        assert_eq!(positive_integer("0"), None);
        assert_eq!(positive_integer("-1"), None);
        assert_eq!(positive_integer("1.5"), None);
        assert_eq!(positive_integer("nope"), None);
    }

    #[test]
    fn interactive_editor_submits_edited_and_historical_lines() {
        fn read(editor: &mut LineEditor, bytes: &[u8]) -> (Option<String>, String) {
            let mut out = Vec::new();
            let line = editor
                .read_interactive(&mut std::io::Cursor::new(bytes), &mut out, "> ")
                .expect("the editor cannot fail");
            (line, String::from_utf8(out).expect("the redraw is UTF-8"))
        }

        let mut editor = LineEditor::new(true);
        let (line, drawn) = read(&mut editor, b"hi\x1b[D!\r");
        assert_eq!(line.as_deref(), Some("h!i"));
        assert!(
            drawn.contains("\u{1b}[1D"),
            "the redraw moves the cursor back: {drawn:?}"
        );

        // Up recalls the most recent line; Enter submits the recalled text.
        let (recalled, _) = read(&mut editor, b"\x1b[A\r");
        assert_eq!(recalled.as_deref(), Some("h!i"));

        // Up then Down restores the empty draft; typing submits fresh text.
        let (draft, _) = read(&mut editor, b"\x1b[A\x1b[Bx\r");
        assert_eq!(draft.as_deref(), Some("x"));

        // Ctrl+C is exit intent, not a submitted line.
        let (interrupted, _) = read(&mut editor, b"\x03");
        assert_eq!(interrupted, None);

        assert_eq!(
            editor.history,
            vec!["h!i".to_string(), "h!i".to_string(), "x".to_string()]
        );
    }
}
