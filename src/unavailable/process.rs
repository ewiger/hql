//! Child processes, in a build that starts none.

use super::reason;
use crate::io::CommandSpec;

/// Why a command did not succeed; never constructed in this build.
#[derive(Debug, Clone, Default)]
pub(crate) struct Failure {
    pub(crate) reason: &'static str,
    pub(crate) status: Option<i32>,
    pub(crate) signal: Option<i32>,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Why an operation on a command stopped: always a refusal in this build.
#[derive(Debug, Clone)]
pub(crate) enum Problem {
    Refused(String),
    #[allow(dead_code)]
    Failed(Failure),
}

impl Problem {
    pub(crate) fn message(&self, _spec: &CommandSpec) -> String {
        match self {
            Self::Refused(message) => message.clone(),
            Self::Failed(failure) => failure.reason.to_owned(),
        }
    }
}

/// What a child that exited `0` wrote; never constructed in this build.
pub(crate) struct Ran {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// A child's output; never constructed in this build.
pub(crate) enum Output {}

impl std::io::Read for Output {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        match *self {}
    }
}

/// A child's input; never constructed in this build.
pub(crate) enum Input {}

impl Input {
    pub(crate) fn line(&mut self, _text: &str) -> Result<(), Problem> {
        match *self {}
    }

    pub(crate) fn finish(&mut self) -> Result<(), Problem> {
        match *self {}
    }
}

/// A reader that replaces text that is not UTF-8; it passes bytes through
/// in this build, where no child writes any.
pub(crate) struct Lossy<R>(R);

impl<R> Lossy<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self(inner)
    }
}

impl<R: std::io::Read> std::io::Read for Lossy<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buffer)
    }
}

pub(crate) fn signal_name(number: i32) -> String {
    number.to_string()
}

pub(crate) fn run(_spec: &CommandSpec, _input: &[u8]) -> Result<Ran, Problem> {
    Err(Problem::Refused(reason("a child process")))
}

pub(crate) fn output(_spec: &CommandSpec) -> Result<Output, Problem> {
    Err(Problem::Refused(reason("a child process")))
}

pub(crate) fn input(_spec: &CommandSpec) -> Result<Input, Problem> {
    Err(Problem::Refused(reason("a child process")))
}

pub(crate) fn text(bytes: Vec<u8>) -> Option<String> {
    String::from_utf8(bytes).ok()
}
