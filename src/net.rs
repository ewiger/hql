//! The network at the edge: HTTP requests, WebSocket connections and TCP
//! sockets, for the endpoints of `import net`.
//!
//! Nothing here is reached by constructing an endpoint. A request is sent and
//! a connection opened only when [`crate::io`] reads or writes one. See
//! [HQL-0021](../doc/proposals/HQL-0021/README.md).
//!
//! Every connection here is bounded, as HQL-0047 requires of a `Network`
//! transport: an attempt to connect fails after [`CONNECT`], and an HTTP
//! response or a WebSocket handshake that sends nothing for [`RESPONSE`]
//! fails, each naming the URL or the address. A live read, a message on a
//! WebSocket or a line on a TCP stream, waits without limit, as a `Stream`
//! does.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// How long an attempt to connect may take before it fails.
pub(crate) const CONNECT: Duration = Duration::from_secs(10);

/// How long an HTTP response, or a WebSocket handshake, may send nothing
/// before it fails; `timeout_ms` overrides it for one `http` endpoint. A
/// write that is not accepted for this long fails too.
pub(crate) const RESPONSE: Duration = Duration::from_secs(30);

/// A live WebSocket connection, over TLS or not.
pub(crate) type Connection = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

/// A bound as a failure states it: whole seconds where it is a whole
/// number of them, milliseconds otherwise.
fn shown(bound: Duration) -> String {
    if bound.subsec_millis() == 0 {
        format!("{} s", bound.as_secs())
    } else {
        format!("{} ms", bound.as_millis())
    }
}

/// Whether an I/O failure is a bound running out: a socket timeout surfaces
/// as `TimedOut` or, on Unix, as `WouldBlock`.
fn ran_out(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

/// Why a connection failed, the bound named when it ran out.
fn unreached(error: &std::io::Error, connect: Duration) -> String {
    if ran_out(error) {
        format!("no connection within {}", shown(connect))
    } else {
        error.to_string()
    }
}

/// Why a peer went silent: nothing read, or nothing written, for `bound`.
fn silent(bound: Duration) -> String {
    format!("the server sent nothing for {}", shown(bound))
}

/// A write the peer did not accept for `bound`.
fn unaccepted(bound: Duration) -> String {
    format!("a write was not accepted for {}", shown(bound))
}

/// An I/O failure in a transport's own source chain, if it holds one.
fn io_cause<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a std::io::Error> {
    let mut cause = Some(error);
    while let Some(current) = cause {
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            return Some(io);
        }
        cause = current.source();
    }
    None
}

/// An HTTP client whose connection and response are bounded.
fn agent(response: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(CONNECT)
        .timeout_read(response)
        .timeout_write(response)
        .build()
}

fn request(
    method: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    response: Duration,
) -> ureq::Request {
    headers.iter().fold(
        agent(response).request(method, url),
        |request, (name, value)| request.set(name, value),
    )
}

/// Why a request failed, naming the URL and, when there was one, the status.
fn failure(url: &str, error: ureq::Error, response: Duration) -> String {
    match error {
        ureq::Error::Status(status, answer) => {
            format!(
                "{url}: the server answered {status} {}",
                answer.status_text()
            )
        }
        ureq::Error::Transport(transport) => match io_cause(&transport) {
            Some(io) if ran_out(io) => match transport.kind() {
                ureq::ErrorKind::ConnectionFailed => format!("{url}: {}", unreached(io, CONNECT)),
                _ => format!("{url}: {}", silent(response)),
            },
            _ => format!("{url}: {transport}"),
        },
    }
}

/// The body of a `GET`, the response failing once it sends nothing for
/// `response`.
///
/// # Errors
/// Names the URL, with the status when the server answered one that is not a
/// success, and the bound when one ran out.
pub(crate) fn fetch(
    url: &str,
    headers: &BTreeMap<String, String>,
    response: Duration,
) -> Result<Vec<u8>, String> {
    let answer = request("GET", url, headers, response)
        .call()
        .map_err(|error| failure(url, error, response))?;
    let mut body = Vec::new();
    answer
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|error| {
            if ran_out(&error) {
                format!("{url}: {}", silent(response))
            } else {
                format!("{url}: {error}")
            }
        })?;
    Ok(body)
}

/// Send one request whose body is `text`; the response is not read beyond
/// its status, and fails once it sends nothing for `response`.
///
/// # Errors
/// Names the URL, with the status when it is not a success and the bound
/// when one ran out.
pub(crate) fn send(
    url: &str,
    method: &str,
    headers: &BTreeMap<String, String>,
    text: &str,
    response: Duration,
) -> Result<(), String> {
    request(method, url, headers, response)
        .send_string(text)
        .map(drop)
        .map_err(|error| failure(url, error, response))
}

/// A TCP connection to the first address of `host` that answers, every
/// attempt together bounded by `within`.
fn connected(host: &str, port: u16, within: Duration) -> std::io::Result<TcpStream> {
    let deadline = Instant::now() + within;
    let mut last = None;
    for address in (host, port).to_socket_addrs()? {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        match TcpStream::connect_timeout(&address, left) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "the host has no address")
    }))
}

/// A TCP connection. A read waits without limit; a write the peer does not
/// accept for [`RESPONSE`] fails.
///
/// # Errors
/// Names the address when it cannot be reached, or not within [`CONNECT`].
pub(crate) fn socket(host: &str, port: u16) -> Result<TcpStream, String> {
    let stream = connected(host, port, CONNECT)
        .map_err(|error| format!("{host}:{port}: {}", unreached(&error, CONNECT)))?;
    stream
        .set_write_timeout(Some(RESPONSE))
        .map_err(|error| format!("{host}:{port}: {error}"))?;
    Ok(stream)
}

/// A WebSocket connection, bounded by [`CONNECT`] and, for its handshake,
/// by [`RESPONSE`].
///
/// # Errors
/// Names the URL when the handshake fails or a bound runs out.
pub(crate) fn websocket(url: &str) -> Result<Connection, String> {
    websocket_within(url, CONNECT, RESPONSE)
}

/// A WebSocket connection whose attempt is bounded by `connect` and whose
/// handshake fails once the server sends nothing for `response`. Once open,
/// a read waits without limit and a write the server does not accept for
/// `response` fails.
fn websocket_within(
    url: &str,
    connect: Duration,
    response: Duration,
) -> Result<Connection, String> {
    use tungstenite::client::IntoClientRequest;
    use tungstenite::handshake::HandshakeError;
    use tungstenite::stream::Mode;
    let named = |error: &dyn std::fmt::Display| format!("{url}: {error}");
    let request = url.into_client_request().map_err(|error| named(&error))?;
    let uri = request.uri();
    let port = uri.port_u16().unwrap_or(
        match tungstenite::client::uri_mode(uri).map_err(|error| named(&error))? {
            Mode::Plain => 80,
            Mode::Tls => 443,
        },
    );
    let host = uri
        .host()
        .map(|host| host.trim_start_matches('[').trim_end_matches(']'))
        .ok_or_else(|| format!("{url}: the URL names no host"))?;
    let stream = connected(host, port, connect)
        .map_err(|error| format!("{url}: {}", unreached(&error, connect)))?;
    // The clone is the same socket: the handshake's read bound is lifted
    // through it once the handshake is done.
    let held = stream.try_clone().map_err(|error| named(&error))?;
    stream
        .set_nodelay(true)
        .and_then(|()| stream.set_read_timeout(Some(response)))
        .and_then(|()| stream.set_write_timeout(Some(response)))
        .map_err(|error| named(&error))?;
    let connection = match tungstenite::client_tls(request, stream) {
        Ok((connection, _)) => connection,
        Err(HandshakeError::Interrupted(_)) => return Err(format!("{url}: {}", silent(response))),
        Err(HandshakeError::Failure(tungstenite::Error::Io(error))) if ran_out(&error) => {
            return Err(format!("{url}: {}", silent(response)));
        }
        Err(HandshakeError::Failure(error)) => return Err(named(&error)),
    };
    held.set_read_timeout(None).map_err(|error| named(&error))?;
    Ok(connection)
}

/// The next message's text, or `None` once the connection is closed.
///
/// # Errors
/// Names the URL when the connection fails other than by closing.
pub(crate) fn message(connection: &mut Connection, url: &str) -> Option<Result<String, String>> {
    use tungstenite::{Error, Message};
    loop {
        return match connection.read() {
            Ok(Message::Text(text)) => Some(Ok(text.as_str().to_owned())),
            Ok(Message::Binary(bytes)) => Some(
                String::from_utf8(bytes.to_vec())
                    .map_err(|_| format!("{url}: a message is not UTF-8 text")),
            ),
            Ok(Message::Close(_)) | Err(Error::ConnectionClosed | Error::AlreadyClosed) => None,
            Ok(_) => continue,
            Err(error) => Some(Err(format!("{url}: {error}"))),
        };
    }
}

/// Send one document as one message.
///
/// # Errors
/// Names the URL when the message cannot be sent, or is not accepted within
/// [`RESPONSE`].
pub(crate) fn post(connection: &mut Connection, url: &str, text: &str) -> Result<(), String> {
    connection
        .send(tungstenite::Message::Text(text.to_owned().into()))
        .map_err(|error| match error {
            tungstenite::Error::Io(io) if ran_out(&io) => {
                format!("{url}: {}", unaccepted(RESPONSE))
            }
            other => format!("{url}: {other}"),
        })
}

/// Write one document and a line break to a socket.
///
/// # Errors
/// Names the address when the write fails, or is not accepted within
/// [`RESPONSE`].
pub(crate) fn line(stream: &mut TcpStream, text: &str) -> Result<(), String> {
    writeln!(stream, "{text}")
        .and_then(|()| stream.flush())
        .map_err(|error| {
            let peer = stream
                .peer_addr()
                .map_or_else(|_| "socket".to_owned(), |address| address.to_string());
            if ran_out(&error) {
                format!("{peer}: {}", unaccepted(RESPONSE))
            } else {
                format!("{peer}: {error}")
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    /// A loopback server that accepts one connection and sends nothing
    /// until the test is done with it.
    fn silent_server() -> (u16, mpsc::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (done, wait) = mpsc::channel::<()>();
        thread::spawn(move || {
            let held = listener.accept();
            let _ = wait.recv();
            drop(held);
        });
        (port, done)
    }

    #[test]
    fn a_bound_is_shown_in_seconds_or_milliseconds() {
        assert_eq!(shown(CONNECT), "10 s");
        assert_eq!(shown(RESPONSE), "30 s");
        assert_eq!(shown(Duration::from_millis(250)), "250 ms");
    }

    #[test]
    fn a_silent_websocket_handshake_fails_naming_the_url() {
        let (port, done) = silent_server();
        let url = format!("ws://127.0.0.1:{port}/events");
        let started = Instant::now();
        let error = websocket_within(&url, CONNECT, Duration::from_millis(200))
            .err()
            .unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(error, format!("{url}: the server sent nothing for 200 ms"));
        drop(done);
    }

    #[test]
    fn a_silent_http_response_fails_naming_the_url() {
        let (port, done) = silent_server();
        let url = format!("http://127.0.0.1:{port}/birds");
        let error = fetch(&url, &BTreeMap::new(), Duration::from_millis(200))
            .err()
            .unwrap();
        assert_eq!(error, format!("{url}: the server sent nothing for 200 ms"));
        drop(done);
    }

    #[test]
    fn a_refused_connection_names_the_address() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let error = socket("127.0.0.1", port).err().unwrap();
        assert!(error.starts_with(&format!("127.0.0.1:{port}: ")), "{error}");
    }
}
