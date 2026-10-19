//! The network at the edge: HTTP requests, WebSocket connections and TCP
//! sockets, for the endpoints of `import net`.
//!
//! Nothing here is reached by constructing an endpoint. A request is sent and
//! a connection opened only when [`crate::io`] reads or writes one. See
//! [HQL-0021](../doc/proposals/HQL-0021/README.md).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

/// A live WebSocket connection, over TLS or not.
pub(crate) type Connection = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

fn request(method: &str, url: &str, headers: &BTreeMap<String, String>) -> ureq::Request {
    headers
        .iter()
        .fold(ureq::request(method, url), |request, (name, value)| {
            request.set(name, value)
        })
}

/// Why a request failed, naming the URL and, when there was one, the status.
fn failure(url: &str, error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(status, response) => {
            format!(
                "{url}: the server answered {status} {}",
                response.status_text()
            )
        }
        ureq::Error::Transport(transport) => format!("{url}: {transport}"),
    }
}

/// The body of a `GET`.
///
/// # Errors
/// Names the URL, with the status when the server answered one that is not a
/// success.
pub(crate) fn fetch(url: &str, headers: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
    let response = request("GET", url, headers)
        .call()
        .map_err(|error| failure(url, error))?;
    let mut body = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|error| format!("{url}: {error}"))?;
    Ok(body)
}

/// Send one request whose body is `text`; the response is not read beyond
/// its status.
///
/// # Errors
/// Names the URL, with the status when it is not a success.
pub(crate) fn send(
    url: &str,
    method: &str,
    headers: &BTreeMap<String, String>,
    text: &str,
) -> Result<(), String> {
    request(method, url, headers)
        .send_string(text)
        .map(drop)
        .map_err(|error| failure(url, error))
}

/// A TCP connection.
///
/// # Errors
/// Names the address when it cannot be reached.
pub(crate) fn socket(host: &str, port: u16) -> Result<TcpStream, String> {
    TcpStream::connect((host, port)).map_err(|error| format!("{host}:{port}: {error}"))
}

/// A WebSocket connection.
///
/// # Errors
/// Names the URL when the handshake fails.
pub(crate) fn websocket(url: &str) -> Result<Connection, String> {
    tungstenite::connect(url)
        .map(|(connection, _)| connection)
        .map_err(|error| format!("{url}: {error}"))
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
/// Names the URL when the message cannot be sent.
pub(crate) fn post(connection: &mut Connection, url: &str, text: &str) -> Result<(), String> {
    connection
        .send(tungstenite::Message::Text(text.to_owned().into()))
        .map_err(|error| format!("{url}: {error}"))
}

/// Write one document and a line break to a socket.
///
/// # Errors
/// Names the address when the write fails.
pub(crate) fn line(stream: &mut TcpStream, text: &str) -> Result<(), String> {
    writeln!(stream, "{text}")
        .and_then(|()| stream.flush())
        .map_err(|error| {
            let peer = stream
                .peer_addr()
                .map_or_else(|_| "socket".to_owned(), |address| address.to_string());
            format!("{peer}: {error}")
        })
}
