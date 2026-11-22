//! Child processes (HQL-0064, the companion note on child processes): a
//! [`CommandSpec`] started, written, read and waited for.
//!
//! Every child is started as the leader of a process group of its own, and
//! terminating it signals that group, so the processes it started end with
//! it: `SIGTERM`, then `SIGKILL` after five seconds. A child that runs past
//! its `timeout_ms` is terminated by a guard thread. A child whose reader or
//! writer is dropped before it ended, because a stream was not read to its
//! end, is terminated and waited for.
//!
//! The functions here run on any thread: a [`CommandSpec`] holds every path
//! already resolved, and nothing here reads the evaluation's host state.

use crate::io::CommandSpec;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

/// How long a terminated child has to end before it is killed.
const GRACE: Duration = Duration::from_secs(5);

/// How many bytes of a child's standard error a failure keeps.
const KEPT: usize = 4096;

/// Why a command did not succeed: the fields of `CommandFailure`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Failure {
    /// `not-found`, `not-permitted`, `exited`, `signalled`, `timed-out` or
    /// `closed`.
    pub(crate) reason: &'static str,
    /// The exit status, for `exited`.
    pub(crate) status: Option<i32>,
    /// The number of the signal that ended the child, for `signalled`.
    pub(crate) signal: Option<i32>,
    /// What the child wrote to its standard output before it ended, where
    /// it was captured.
    pub(crate) stdout: Vec<u8>,
    /// What the child wrote to its standard error before it ended: all of it
    /// under `run`, its last bytes where it was passed on.
    pub(crate) stderr: Vec<u8>,
    /// What the operating system said, for a child that did not start.
    pub(crate) detail: String,
}

impl Failure {
    fn new(reason: &'static str) -> Self {
        Self {
            reason,
            ..Self::default()
        }
    }

    /// The failure as one message: the command, the reason, the status or
    /// signal, and the last lines of standard error.
    pub(crate) fn message(&self, spec: &CommandSpec) -> String {
        format!("command `{}` failed: {}", spec.program, self.detail())
    }

    /// The failure without the command's name, for a reader whose errors
    /// are already prefixed with the endpoint.
    fn detail(&self) -> String {
        let mut message = self.reason.to_owned();
        if let Some(status) = self.status {
            message.push_str(&format!(", status {status}"));
        }
        if let Some(signal) = self.signal {
            message.push_str(&format!(", signal {}", signal_name(signal)));
        }
        if !self.detail.is_empty() {
            message.push_str(&format!(" ({})", self.detail));
        }
        let stderr = String::from_utf8_lossy(&self.stderr);
        let last: Vec<&str> = stderr.lines().rev().take(5).collect();
        if !last.is_empty() {
            let last: Vec<&str> = last.into_iter().rev().collect();
            message.push_str(&format!(
                "; its standard error ended with:\n{}",
                last.join("\n")
            ));
        }
        message
    }
}

/// Why an operation on a command stopped.
#[derive(Debug, Clone)]
pub(crate) enum Problem {
    /// The host does not grant the program: a forbidden effect.
    Refused(String),
    /// The command started, or tried to, and did not succeed.
    Failed(Failure),
}

impl Problem {
    /// The problem as an unrecoverable failure's message.
    pub(crate) fn message(&self, spec: &CommandSpec) -> String {
        match self {
            Self::Refused(message) => message.clone(),
            Self::Failed(failure) => failure.message(spec),
        }
    }
}

/// What a child that exited `0` wrote.
pub(crate) struct Ran {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// The POSIX name of signal `number`, without `SIG`, for the numbers every
/// Unix gives the same meaning; the number itself for any other.
pub(crate) fn signal_name(number: i32) -> String {
    let name = match number {
        1 => "HUP",
        2 => "INT",
        3 => "QUIT",
        4 => "ILL",
        6 => "ABRT",
        8 => "FPE",
        9 => "KILL",
        11 => "SEGV",
        13 => "PIPE",
        14 => "ALRM",
        15 => "TERM",
        other => return other.to_string(),
    };
    name.to_owned()
}

/// The program a spec executes: a name without `/` found in this process's
/// `PATH`, and a path as it was resolved.
fn executable(spec: &CommandSpec) -> Result<PathBuf, Failure> {
    if spec.program.contains('/') || spec.program.contains(std::path::MAIN_SEPARATOR) {
        return Ok(spec.executable.clone());
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|directory| directory.join(&spec.program))
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            cfg!(windows)
                .then(|| {
                    std::env::split_paths(&path)
                        .map(|directory| directory.join(format!("{}.exe", spec.program)))
                        .find(|candidate| candidate.is_file())
                })
                .flatten()
        })
        .ok_or_else(|| Failure {
            detail: "no program of that name in PATH".to_owned(),
            ..Failure::new("not-found")
        })
}

/// A started child, with the guard that terminates it at its timeout.
struct Running {
    child: Child,
    /// Dropped when the child has ended, which stops the guard.
    ended: Option<mpsc::Sender<()>>,
    guard: Option<JoinHandle<()>>,
    timed_out: Arc<AtomicBool>,
    waited: Option<ExitStatus>,
}

/// Start `spec` with the three standard streams given.
fn start(
    spec: &CommandSpec,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<Running, Problem> {
    crate::extensions::process::permitted(spec).map_err(Problem::Refused)?;
    let program = executable(spec).map_err(Problem::Failed)?;
    let mut command = std::process::Command::new(&program);
    command
        .args(&spec.args)
        .env_clear()
        .envs(crate::extensions::env::granted())
        .envs(&spec.env)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr);
    if let Some(dir) = &spec.dir {
        command.current_dir(dir);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = command.spawn().map_err(|error| {
        let reason = match error.kind() {
            std::io::ErrorKind::NotFound => "not-found",
            _ => "not-permitted",
        };
        Problem::Failed(Failure {
            detail: error.to_string(),
            ..Failure::new(reason)
        })
    })?;
    let timed_out = Arc::new(AtomicBool::new(false));
    let (ended, guard) = match spec.timeout {
        None => (None, None),
        Some(timeout) => {
            let (sender, receiver) = mpsc::channel::<()>();
            let id = child.id();
            let flag = Arc::clone(&timed_out);
            let guard = std::thread::spawn(move || {
                if receiver.recv_timeout(timeout) == Err(mpsc::RecvTimeoutError::Timeout) {
                    flag.store(true, Ordering::SeqCst);
                    stop(id, false);
                    if receiver.recv_timeout(GRACE) == Err(mpsc::RecvTimeoutError::Timeout) {
                        stop(id, true);
                    }
                }
            });
            (Some(sender), Some(guard))
        }
    };
    Ok(Running {
        child,
        ended,
        guard,
        timed_out,
        waited: None,
    })
}

/// Signal the process group led by `id`: `SIGTERM`, or `SIGKILL` when
/// `kill`. A group that has already ended is left as it is.
#[cfg(unix)]
fn stop(id: u32, kill: bool) {
    let signal = if kill {
        hql_sys::Stop::Kill
    } else {
        hql_sys::Stop::Term
    };
    let _ = hql_sys::stop_group(id, signal);
}

/// Without process groups, the guard cannot reach the child; the owner
/// ends it when it is dropped.
#[cfg(not(unix))]
fn stop(_id: u32, _kill: bool) {}

impl Running {
    /// Wait for the child to end, and stop its guard.
    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        if let Some(status) = self.waited {
            return Ok(status);
        }
        let status = self.child.wait()?;
        self.waited = Some(status);
        self.ended = None;
        if let Some(guard) = self.guard.take() {
            let _ = guard.join();
        }
        Ok(status)
    }

    /// Terminate the child and its group, and wait for it.
    fn terminate(&mut self) {
        if self.waited.is_some() {
            return;
        }
        #[cfg(unix)]
        {
            stop(self.child.id(), false);
            let deadline = std::time::Instant::now() + GRACE;
            while std::time::Instant::now() < deadline {
                if let Ok(Some(_)) = self.child.try_wait() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            stop(self.child.id(), true);
        }
        #[cfg(not(unix))]
        {
            let _ = self.child.kill();
        }
        let _ = self.wait();
    }

    /// The failure an ended child's status states, if any.
    fn failure(&self, status: ExitStatus) -> Option<Failure> {
        if self.timed_out.load(Ordering::SeqCst) {
            return Some(Failure::new("timed-out"));
        }
        if status.success() {
            return None;
        }
        if let Some(code) = status.code() {
            return Some(Failure {
                status: Some(code),
                ..Failure::new("exited")
            });
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Some(Failure {
                    signal: Some(signal),
                    ..Failure::new("signalled")
                });
            }
        }
        Some(Failure::new("signalled"))
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Read a pipe to its end on a thread of its own.
fn drained<R: Read + Send + 'static>(mut pipe: R) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// Pass a child's standard error on to this process's, as it arrives, and
/// keep its last bytes for a failure to state.
fn passed_on(mut pipe: ChildStderr) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut kept: VecDeque<u8> = VecDeque::new();
        let mut buffer = [0_u8; 4096];
        let mut err = std::io::stderr();
        while let Ok(count) = pipe.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let _ = err.write_all(&buffer[..count]);
            let _ = err.flush();
            kept.extend(&buffer[..count]);
            while kept.len() > KEPT {
                kept.pop_front();
            }
        }
        kept.into_iter().collect()
    })
}

/// Start `spec`, write `input` to its standard input, close it, and wait,
/// with both outputs captured.
pub(crate) fn run(spec: &CommandSpec, input: &[u8]) -> Result<Ran, Problem> {
    let mut running = start(spec, Stdio::piped(), Stdio::piped(), Stdio::piped())?;
    let stdin = running.child.stdin.take();
    let input = input.to_vec();
    // The input is written on a thread of its own, so a child that writes
    // before it reads does not hold the write up. A child that ends without
    // reading it is judged by its exit status, not by the unread input.
    let writer = std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            let _ = stdin.write_all(&input);
        }
    });
    let stdout = running.child.stdout.take().map(drained);
    let stderr = running.child.stderr.take().map(drained);
    let status = running.wait().map_err(|error| {
        Problem::Failed(Failure {
            detail: error.to_string(),
            ..Failure::new("not-permitted")
        })
    })?;
    let _ = writer.join();
    let stdout = stdout
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    let stderr = stderr
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    match running.failure(status) {
        None => Ok(Ran { stdout, stderr }),
        Some(failure) => Err(Problem::Failed(Failure {
            stdout,
            stderr,
            ..failure
        })),
    }
}

/// A child's standard output, read as it writes it. Its standard input is
/// closed and its standard error passed on. At the end of the output the
/// child is waited for, and a failure is the read's error.
pub(crate) struct Output {
    running: Running,
    stdout: ChildStdout,
    stderr: Option<JoinHandle<Vec<u8>>>,
    done: bool,
}

/// Start `spec` to be read.
pub(crate) fn output(spec: &CommandSpec) -> Result<Output, Problem> {
    let mut running = start(spec, Stdio::null(), Stdio::piped(), Stdio::piped())?;
    let stdout = running.child.stdout.take().ok_or_else(|| {
        Problem::Refused(format!("command `{}`: no standard output", spec.program))
    })?;
    let stderr = running.child.stderr.take().map(passed_on);
    Ok(Output {
        running,
        stdout,
        stderr,
        done: false,
    })
}

impl Read for Output {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.done {
            return Ok(0);
        }
        let count = self.stdout.read(buffer)?;
        if count > 0 || buffer.is_empty() {
            return Ok(count);
        }
        self.done = true;
        let status = self.running.wait()?;
        let stderr = self
            .stderr
            .take()
            .and_then(|thread| thread.join().ok())
            .unwrap_or_default();
        match self.running.failure(status) {
            None => Ok(0),
            Some(failure) => Err(std::io::Error::other(format!(
                "failed: {}",
                Failure { stderr, ..failure }.detail()
            ))),
        }
    }
}

/// A child's standard input, written one document at a time. Its standard
/// output goes to this process's and its standard error is passed on.
pub(crate) struct Input {
    running: Running,
    stdin: Option<ChildStdin>,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

/// Start `spec` to be written.
pub(crate) fn input(spec: &CommandSpec) -> Result<Input, Problem> {
    let mut running = start(spec, Stdio::piped(), Stdio::inherit(), Stdio::piped())?;
    let stdin = running.child.stdin.take();
    let stderr = running.child.stderr.take().map(passed_on);
    Ok(Input {
        running,
        stdin,
        stderr,
    })
}

impl Input {
    /// Write one document and a line break.
    pub(crate) fn line(&mut self, text: &str) -> Result<(), Problem> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(Problem::Failed(Failure::new("closed")));
        };
        let written = stdin
            .write_all(text.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush());
        match written {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                self.stdin = None;
                let _ = self.running.wait();
                Err(Problem::Failed(Failure {
                    stderr: self.kept(),
                    ..Failure::new("closed")
                }))
            }
            Err(error) => Err(Problem::Failed(Failure {
                detail: error.to_string(),
                ..Failure::new("closed")
            })),
        }
    }

    /// Close the child's standard input and wait for it.
    pub(crate) fn finish(&mut self) -> Result<(), Problem> {
        self.stdin = None;
        let status = self.running.wait().map_err(|error| {
            Problem::Failed(Failure {
                detail: error.to_string(),
                ..Failure::new("not-permitted")
            })
        })?;
        match self.running.failure(status) {
            None => Ok(()),
            Some(failure) => Err(Problem::Failed(Failure {
                stderr: self.kept(),
                ..failure
            })),
        }
    }

    fn kept(&mut self) -> Vec<u8> {
        self.stderr
            .take()
            .and_then(|thread| thread.join().ok())
            .unwrap_or_default()
    }
}

/// A reader that gives text with each sequence that is not UTF-8 replaced
/// by U+FFFD, the replacement character: `invalid_utf8 = "replace"`.
pub(crate) struct Lossy<R> {
    inner: R,
    pending: Vec<u8>,
    ready: Vec<u8>,
    at: usize,
    ended: bool,
}

impl<R: Read> Lossy<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self {
            inner,
            pending: Vec::new(),
            ready: Vec::new(),
            at: 0,
            ended: false,
        }
    }

    /// Decode what is pending into `ready`, keeping an incomplete sequence
    /// at its end for the next read unless the input has ended.
    fn decode(&mut self) {
        let bytes = std::mem::take(&mut self.pending);
        let mut rest = bytes.as_slice();
        loop {
            match std::str::from_utf8(rest) {
                Ok(text) => {
                    self.ready.extend_from_slice(text.as_bytes());
                    break;
                }
                Err(error) => {
                    let (valid, after) = rest.split_at(error.valid_up_to());
                    self.ready.extend_from_slice(valid);
                    match error.error_len() {
                        Some(length) => {
                            self.ready.extend_from_slice("\u{FFFD}".as_bytes());
                            rest = &after[length..];
                        }
                        None if self.ended => {
                            self.ready.extend_from_slice("\u{FFFD}".as_bytes());
                            break;
                        }
                        None => {
                            self.pending = after.to_vec();
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl<R: Read> Read for Lossy<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        while self.at == self.ready.len() {
            self.ready.clear();
            self.at = 0;
            if self.ended && self.pending.is_empty() {
                return Ok(0);
            }
            let mut chunk = [0_u8; 8192];
            let count = self.inner.read(&mut chunk)?;
            if count == 0 {
                self.ended = true;
            }
            self.pending.extend_from_slice(&chunk[..count]);
            self.decode();
        }
        let count = buffer.len().min(self.ready.len() - self.at);
        buffer[..count].copy_from_slice(&self.ready[self.at..self.at + count]);
        self.at += count;
        Ok(count)
    }
}

/// Text from a child's bytes under `invalid_utf8`: `None` where the bytes
/// are not UTF-8 and the setting is `fail`.
pub(crate) fn text(bytes: Vec<u8>) -> Option<String> {
    match String::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(error) => match crate::extensions::process::invalid_utf8() {
            crate::extensions::process::InvalidUtf8::Fail => None,
            crate::extensions::process::InvalidUtf8::Replace => {
                Some(String::from_utf8_lossy(error.as_bytes()).into_owned())
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::Lossy;
    use std::io::Read;

    fn lossy(bytes: &[u8]) -> String {
        let mut text = String::new();
        Lossy::new(bytes).read_to_string(&mut text).unwrap();
        text
    }

    #[test]
    fn valid_text_passes_unchanged() {
        assert_eq!(lossy("wren\nkite ü\n".as_bytes()), "wren\nkite ü\n");
    }

    #[test]
    fn an_invalid_byte_becomes_the_replacement_character() {
        assert_eq!(lossy(b"wr\xffen"), "wr\u{FFFD}en");
    }

    #[test]
    fn a_sequence_cut_at_the_end_is_replaced() {
        assert_eq!(lossy(b"kite\xc3"), "kite\u{FFFD}");
    }
}
