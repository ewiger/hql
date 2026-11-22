//! HTTP, WebSocket and TCP, in a build that reaches no server.

use super::reason;
use std::collections::BTreeMap;
use std::net::TcpStream;
use std::time::Duration;

/// How long a connection may take; kept so a configured timeout reads the
/// same in both builds.
#[allow(dead_code)]
pub(crate) const CONNECT: Duration = Duration::from_secs(10);

/// How long an answer may take, unless `timeout_ms` is written.
pub(crate) const RESPONSE: Duration = Duration::from_secs(30);

/// An open WebSocket; never constructed in this build.
pub(crate) enum Connection {}

pub(crate) fn fetch(
    _url: &str,
    _headers: &BTreeMap<String, String>,
    _response: Duration,
) -> Result<Vec<u8>, String> {
    Err(reason("an HTTP request"))
}

pub(crate) fn send(
    _url: &str,
    _method: &str,
    _headers: &BTreeMap<String, String>,
    _text: &str,
    _response: Duration,
) -> Result<(), String> {
    Err(reason("an HTTP request"))
}

pub(crate) fn socket(_host: &str, _port: u16) -> Result<TcpStream, String> {
    Err(reason("a TCP connection"))
}

pub(crate) fn websocket(_url: &str) -> Result<Connection, String> {
    Err(reason("a WebSocket"))
}

pub(crate) fn message(connection: &mut Connection, _url: &str) -> Option<Result<String, String>> {
    match *connection {}
}

pub(crate) fn post(connection: &mut Connection, _url: &str, _text: &str) -> Result<(), String> {
    match *connection {}
}

pub(crate) fn line(_stream: &mut TcpStream, _text: &str) -> Result<(), String> {
    Err(reason("a TCP connection"))
}
