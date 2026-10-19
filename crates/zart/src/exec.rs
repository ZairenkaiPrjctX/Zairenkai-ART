//! Running Android shell tools with an executable allowlist, argument validation, a timeout and
//! an output cap. There is no shell: every caller passes a fixed argv built from validated values.

use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_OUTPUT: usize = 1024 * 1024;

/// Programs zart may run, and nothing else.
pub const ALLOWED_PROGRAMS: [&str; 2] = ["getprop", "cmd"];

#[derive(Debug)]
pub enum ExecError {
    NotAllowed(String),
    Spawn(io::Error),
    Timeout,
    /// Stdout exceeded [`MAX_OUTPUT`]. Reported instead of parsing a truncated prefix.
    OutputTooLarge,
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::NotAllowed(e) => write!(f, "not allowed: {e}"),
            ExecError::Spawn(e) => write!(f, "spawn failed: {e}"),
            ExecError::Timeout => write!(f, "timed out"),
            ExecError::OutputTooLarge => write!(f, "output larger than {MAX_OUTPUT} bytes"),
        }
    }
}

impl std::error::Error for ExecError {}

/// A finished process. A non-zero exit is not an error here: ART's shell commands report
/// errors on stdout with exit status 1, and callers need that text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Arguments: short, printable, no shell metacharacters (defence in depth; no shell is used).
pub fn valid_arg(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 255
        && a.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

pub fn check_argv(argv: &[&str]) -> Result<(), ExecError> {
    let (prog, args) = argv
        .split_first()
        .ok_or_else(|| ExecError::NotAllowed("empty argv".into()))?;
    if !ALLOWED_PROGRAMS.contains(prog) {
        return Err(ExecError::NotAllowed(format!(
            "program {prog:?} is not on the allowlist"
        )));
    }
    if let Some(a) = args.iter().find(|a| !valid_arg(a)) {
        return Err(ExecError::NotAllowed(format!("argument {a:?} rejected")));
    }
    Ok(())
}

/// Read at most `cap` bytes and drain the rest. The flag is true when more than `cap` bytes came.
fn read_capped(mut r: impl Read, cap: usize) -> (Vec<u8>, bool) {
    let mut buf = Vec::new();
    let _ = (&mut r).take(cap as u64 + 1).read_to_end(&mut buf);
    let over = buf.len() > cap;
    buf.truncate(cap);
    let _ = io::copy(&mut r, &mut io::sink());
    (buf, over)
}

pub trait Runner {
    fn run(&self, argv: &[&str], timeout: Duration) -> Result<Output, ExecError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, argv: &[&str], timeout: Duration) -> Result<Output, ExecError> {
        check_argv(argv)?;
        let mut child = Command::new(argv[0])
            .args(&argv[1..])
            .env_clear()
            .env("PATH", "/system/bin:/system/xbin:/vendor/bin:/usr/bin:/bin")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ExecError::Spawn)?;
        let out = child.stdout.take().expect("piped");
        let err = child.stderr.take().expect("piped");
        let reader = thread::spawn(move || read_capped(out, MAX_OUTPUT));
        let err_reader = thread::spawn(move || read_capped(err, 16 * 1024).0);
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait().map_err(ExecError::Spawn)? {
                Some(s) => break s,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ExecError::Timeout);
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        };
        let (stdout, too_large) = reader.join().unwrap_or_default();
        if too_large {
            return Err(ExecError::OutputTooLarge);
        }
        Ok(Output {
            code: status.code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&err_reader.join().unwrap_or_default())
                .chars()
                .take(2000)
                .collect(),
        })
    }
}

/// Canned outputs keyed by the joined argv, with the same allowlist check as production.
#[derive(Debug, Default)]
pub struct FakeRunner {
    outputs: std::sync::Mutex<std::collections::HashMap<String, Result<Output, String>>>,
    calls: std::sync::Mutex<Vec<String>>,
}

impl FakeRunner {
    pub fn with(self, argv: &str, code: i32, stdout: &str) -> Self {
        self.outputs.lock().unwrap().insert(
            argv.to_string(),
            Ok(Output {
                code: Some(code),
                stdout: stdout.to_string(),
                stderr: String::new(),
            }),
        );
        self
    }

    /// `"timeout"` simulates a timeout, `"too_large"` oversized output; anything else a spawn
    /// failure.
    pub fn failing(self, argv: &str, what: &str) -> Self {
        self.outputs
            .lock()
            .unwrap()
            .insert(argv.to_string(), Err(what.to_string()));
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl Runner for FakeRunner {
    fn run(&self, argv: &[&str], _timeout: Duration) -> Result<Output, ExecError> {
        check_argv(argv)?;
        let key = argv.join(" ");
        self.calls.lock().unwrap().push(key.clone());
        match self.outputs.lock().unwrap().get(&key) {
            Some(Ok(o)) => Ok(o.clone()),
            Some(Err(e)) if e == "timeout" => Err(ExecError::Timeout),
            Some(Err(e)) if e == "too_large" => Err(ExecError::OutputTooLarge),
            Some(Err(e)) => Err(ExecError::Spawn(io::Error::other(e.clone()))),
            None => Err(ExecError::Spawn(io::Error::new(
                io::ErrorKind::NotFound,
                key,
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_read_flags_overflow() {
        assert_eq!(read_capped(&b"abc"[..], 3), (b"abc".to_vec(), false));
        assert_eq!(read_capped(&b"abcd"[..], 3), (b"abc".to_vec(), true));
        assert_eq!(read_capped(&b""[..], 3), (Vec::new(), false));
    }

    #[test]
    fn allowlist_and_argument_validation() {
        assert!(check_argv(&["getprop"]).is_ok());
        assert!(check_argv(&["cmd", "package", "art", "dump", "com.example.app"]).is_ok());
        assert!(check_argv(&["sh", "-c", "id"]).is_err());
        assert!(check_argv(&["/system/bin/cmd"]).is_err());
        assert!(check_argv(&["cmd", "package", "art", "dump", "a;reboot"]).is_err());
        assert!(check_argv(&["cmd", "package", "art", "dump", "$(id)"]).is_err());
        assert!(check_argv(&["cmd", "package", "art", "dump", "../x"]).is_err());
        assert!(check_argv(&[]).is_err());
        assert!(matches!(
            SystemRunner.run(&["rm", "-rf", "/tmp/x"], Duration::from_secs(1)),
            Err(ExecError::NotAllowed(_))
        ));
    }
}
