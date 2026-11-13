//! Execution of sample scripts against the real lovelace-run process.
//!
//! There is no mock in this path: every sample spawns the real runner with the script on
//! stdin and the JSON envelope on stdout, exactly as the C# ProcessScriptRunner does. The
//! NullRunner exists only for pure config/reduce paths in unit tests.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::model::{Operand, SampleRecord, SamplingKind};
use crate::observation;
use crate::proposal::SampleSpec;

/// The output of a single runner invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerOutput {
    /// The process exit code.
    pub exit_code: i32,
    /// Everything the runner wrote to stdout.
    pub stdout: String,
    /// Everything the runner wrote to stderr.
    pub stderr: String,
}

/// Abstraction over "execute one Lovelace script and return the JSON envelope".
pub trait ScriptRunner {
    /// Executes one script.
    fn run(&self, script: &str) -> Result<RunnerOutput, String>;
}

/// Spawns the published runner with the script on stdin (--stdin --json).
#[derive(Clone, Debug)]
pub struct ProcessRunner {
    path: PathBuf,
    timeout: Duration,
}

impl ProcessRunner {
    /// Builds a runner for an executable path with the default 30 s timeout.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Builds a runner with an explicit timeout.
    #[must_use]
    pub fn with_timeout(path: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            path: path.into(),
            timeout,
        }
    }

    /// The runner executable path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ScriptRunner for ProcessRunner {
    fn run(&self, script: &str) -> Result<RunnerOutput, String> {
        let mut child = Command::new(&self.path)
            .arg("--stdin")
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot spawn {}: {error}", self.path.display()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(script.as_bytes())
                .map_err(|error| format!("cannot write to the runner: {error}"))?;
        }

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let stdout_handle = thread::spawn(move || read_all(stdout_pipe));
        let stderr_handle = thread::spawn(move || read_all(stderr_pipe));

        let deadline = Instant::now() + self.timeout;
        let exit_code = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code().unwrap_or(-1),
                Ok(None) => {}
                Err(error) => return Err(format!("cannot wait for the runner: {error}")),
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "runner exceeded {} ms for script: {script}",
                    self.timeout.as_millis()
                ));
            }
            thread::sleep(Duration::from_millis(2));
        };

        let stdout =
            String::from_utf8_lossy(&stdout_handle.join().unwrap_or_default()).into_owned();
        let stderr =
            String::from_utf8_lossy(&stderr_handle.join().unwrap_or_default()).into_owned();
        Ok(RunnerOutput {
            exit_code,
            stdout,
            stderr,
        })
    }
}

fn read_all(pipe: Option<impl Read>) -> Vec<u8> {
    let mut buffer = Vec::new();
    if let Some(mut pipe) = pipe {
        let _ = pipe.read_to_end(&mut buffer);
    }
    buffer
}

/// A no-op runner used by the pure config/reduce paths.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullRunner;

impl ScriptRunner for NullRunner {
    fn run(&self, _script: &str) -> Result<RunnerOutput, String> {
        Ok(RunnerOutput {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

/// Executes a batch of specs and canonicalizes each envelope into a sample record.
#[must_use]
pub fn execute_batch(
    runner: &dyn ScriptRunner,
    specs: &[SampleSpec],
    base_index: i32,
) -> Vec<SampleRecord> {
    let mut records = Vec::with_capacity(specs.len());
    for (offset, spec) in specs.iter().enumerate() {
        let output = runner.run(&spec.script);
        let index = base_index + i32::try_from(offset).unwrap_or(0);
        records.push(to_record(spec, index, output));
    }
    records
}

fn to_record(spec: &SampleSpec, index: i32, output: Result<RunnerOutput, String>) -> SampleRecord {
    let observation = match output {
        Ok(output) => canonicalize(&output),
        Err(message) => observation::Observation::failure(message),
    };
    let sigma = observation::plane_sigma(&observation);
    SampleRecord {
        index,
        script: spec.script.clone(),
        op: spec.op,
        left: spec.left.clone(),
        right: spec.right.clone(),
        sweep_id: spec.sweep_id.clone(),
        swept_side: spec.swept_side.clone(),
        axis_pos: spec.axis_pos.clone(),
        sigma,
        success: observation.success,
        kind: observation.kind.clone(),
        typed: observation.typed.clone(),
        error_message: observation.error_message.clone(),
        sampling_kind: spec.sampling_kind,
        weight: spec.weight,
    }
}

fn canonicalize(output: &RunnerOutput) -> observation::Observation {
    // The runner emits the JSON envelope on stdout even for script errors (exit code 1),
    // so parse stdout first regardless of the exit code.
    if !output.stdout.trim().is_empty() {
        if let Ok(observation) = observation::from_runner_output(&output.stdout) {
            return observation;
        }
    }
    let mut message = format!("runner failure (exit {})", output.exit_code);
    let stderr = output.stderr.trim();
    if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    }
    observation::Observation::failure(message)
}

/// Resolves the lovelace-run executable.
///
/// Order: the explicit request field, the LOVELACE_RUN environment variable, an executable
/// next to this one (the cargo target directory), then the usual target directories under
/// the working directory.
pub fn resolve_runner(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    if let Ok(path) = std::env::var("LOVELACE_RUN") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let name = if cfg!(windows) {
        "lovelace-run.exe"
    } else {
        "lovelace-run"
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(name));
            if let Some(parent) = directory.parent() {
                candidates.push(parent.join("release").join(name));
                candidates.push(parent.join("debug").join(name));
            }
        }
    }
    if let Ok(working_directory) = std::env::current_dir() {
        for profile in ["release", "debug"] {
            candidates.push(
                working_directory
                    .join("rust")
                    .join("target")
                    .join(profile)
                    .join(name),
            );
            candidates.push(working_directory.join("target").join(profile).join(name));
        }
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("lovelace-run binary not found; pass the 'runner' argument or set LOVELACE_RUN".to_string())
}

/// Convenience: execute the given scripts and canonicalize them (used by tests).
#[must_use]
pub fn execute_script(runner: &dyn ScriptRunner, script: &str, index: i32) -> SampleRecord {
    let spec = SampleSpec {
        script: script.to_string(),
        op: crate::model::Operation::Add,
        left: Operand::natural("0"),
        right: Operand::natural("0"),
        sweep_id: None,
        swept_side: None,
        axis_pos: None,
        sampling_kind: SamplingKind::Random,
        weight: 1.0,
    };
    to_record(&spec, index, runner.run(script))
}
