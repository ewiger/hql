//! Network endpoints through the real process, against servers on the
//! loopback interface: `http`, `tcp` and `websocket` read and written by
//! `source`, `tap`, `target` and `sink`. See doc/proposals/HQL-0021/README.md.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn hql_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn hql(program: &str) -> Output {
    hql_in(&std::env::temp_dir(), &[program])
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn ok(output: &Output) -> String {
    assert!(output.status.success(), "{}", stderr(output));
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn refused(output: &Output, says: &str) {
    assert!(!output.status.success());
    assert!(stderr(output).contains(says), "{}", stderr(output));
}

/// One HTTP request as the server saw it.
#[derive(Debug)]
struct Request {
    line: String,
    body: String,
}

/// A server that answers `requests` requests, each with `status` and `body`,
/// and reports what it was sent.
fn http_server(
    requests: usize,
    status: &'static str,
    body: &'static str,
) -> (u16, mpsc::Receiver<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sent, received) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming().take(requests) {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut content = vec![0; length];
            reader.read_exact(&mut content).unwrap();
            write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let _ = sent.send(Request {
                line: line.trim().to_owned(),
                body: String::from_utf8(content).unwrap(),
            });
        }
    });
    (port, received)
}

#[test]
fn the_network_is_imported() {
    refused(&hql(r#"http("http://127.0.0.1/")"#), "import net");
    refused(&hql(r#"tcp("127.0.0.1", 1)"#), "import net");
}

#[test]
fn socket_is_refused_naming_tcp() {
    refused(
        &hql("import net\nnet.socket(\"127.0.0.1\", 1)"),
        "`socket` is now `tcp`",
    );
    refused(
        &hql("import net\nfn f(s: Socket) : Int = 1\n1"),
        "`Socket` is now `Tcp`",
    );
    let shown = ok(&hql("import net\nnet.tcp(\"127.0.0.1\", 1)"));
    assert_eq!(shown.trim(), r#"tcp("127.0.0.1", 1)"#);
}

#[test]
fn a_url_is_not_a_file() {
    refused(
        &hql(r#"source(file("https://example.com/birds.json"))"#),
        "network resource is read with `http`",
    );
}

#[test]
fn an_endpoint_is_a_value_that_sends_nothing() {
    let output = hql("import net\n\
         e = http(\"http://127.0.0.1:1/x\", method = \"post\")\n\
         [e.url, e.method]");
    assert_eq!(ok(&output), "[\"http://127.0.0.1:1/x\", \"POST\"]\n");
}

#[test]
fn an_http_resource_is_fetched_once_and_read_as_a_seq() {
    let (port, requests) = http_server(1, "200 OK", "{\"a\": 1}\n{\"a\": 2}\n");
    let output = hql(&format!(
        "import net\n\
         birds = source(http(\"http://127.0.0.1:{port}/birds\"), Json)\n\
         [birds | size, birds | size]"
    ));
    assert_eq!(ok(&output), "[2, 2]\n");
    let request = requests.recv().unwrap();
    assert_eq!(request.line, "GET /birds HTTP/1.1");
    assert!(requests.try_recv().is_err());
}

#[test]
fn a_write_sends_one_request_per_document() {
    let (port, requests) = http_server(2, "200 OK", "");
    let output = hql(&format!(
        "import net\n\
         [1, 2] | sink(http(\"http://127.0.0.1:{port}/n\", method = \"POST\"), Json)"
    ));
    assert_eq!(ok(&output), "");
    let bodies: Vec<String> = requests
        .iter()
        .take(2)
        .map(|request| request.body)
        .collect();
    assert_eq!(bodies, ["1", "2"]);
}

#[test]
fn a_failed_status_names_the_url() {
    let (port, _) = http_server(1, "500 Internal Server Error", "");
    let url = format!("http://127.0.0.1:{port}/report");
    let output = hql(&format!(
        "import net\n{{done: true}} | target(http(\"{url}\", method = \"POST\"), Json)"
    ));
    refused(&output, &format!("{url}: the server answered 500"));
}

/// A server that accepts a connection and never answers, until the test is
/// done with it.
fn silent_server() -> (u16, mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (done, wait) = mpsc::channel::<()>();
    thread::spawn(move || {
        let held: Vec<TcpStream> = listener.incoming().take(1).flatten().collect();
        let _ = wait.recv();
        drop(held);
    });
    (port, done)
}

/// `hql` with a limit of its own: a run still going after `limit` is killed
/// and fails the test, so a hang is a failure and not a stuck suite.
fn hql_within(program: &str, limit: Duration) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hql"))
        .current_dir(std::env::temp_dir())
        .arg(program)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > limit {
            child.kill().unwrap();
            panic!("hql was still running after {limit:?}");
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

#[test]
fn a_silent_server_ends_in_a_failure_naming_the_url() {
    let (port, done) = silent_server();
    let url = format!("http://127.0.0.1:{port}/birds");
    let output = hql_within(
        &format!("import net\nsource(http(\"{url}\", timeout_ms = 200), Json)"),
        Duration::from_secs(20),
    );
    refused(
        &output,
        &format!("{url}: the server sent nothing for 200 ms"),
    );
    drop(done);
}

#[test]
fn a_response_bound_is_above_zero_and_shown_when_written() {
    refused(
        &hql("import net\nhttp(\"http://127.0.0.1:1/\", timeout_ms = 0)"),
        "`timeout_ms` is a count of milliseconds, above 0",
    );
    let shown = ok(&hql(
        "import net\n[http(\"http://127.0.0.1:1/\"), http(\"http://127.0.0.1:1/\", timeout_ms = 500)]",
    ));
    assert_eq!(
        shown.trim(),
        r#"[http("http://127.0.0.1:1/"), http("http://127.0.0.1:1/", timeout_ms = 500)]"#
    );
}

#[test]
fn a_socket_carries_documents_as_their_format_separates_them() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(b"{\"n\": 1}\n{\"n\": 2}\n").unwrap();
        drop(stream);
        let (stream, _) = listener.accept().unwrap();
        let mut written = String::new();
        BufReader::new(stream).read_to_string(&mut written).unwrap();
        written
    });
    let read = hql(&format!(
        "import net\nsource(tcp(\"127.0.0.1\", {port}), Json) | map(d => d.n)"
    ));
    assert_eq!(ok(&read), "1\n2\n");
    let written = hql(&format!(
        "import net\n[\"a\", \"b\"] | sink(tcp(\"127.0.0.1\", {port}), Text)"
    ));
    ok(&written);
    assert_eq!(server.join().unwrap(), "a\nb\n");
}

#[test]
fn a_websocket_message_is_one_document() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let stream: TcpStream = listener.accept().unwrap().0;
        let mut socket = tungstenite::accept(stream).unwrap();
        for text in ["{\"k\": 1}", "{\"k\": 2}"] {
            socket
                .send(tungstenite::Message::Text(text.into()))
                .unwrap();
        }
        socket.close(None).unwrap();
        let _ = socket.flush();
        while socket.read().is_ok() {}
        let stream: TcpStream = listener.accept().unwrap().0;
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut received = Vec::new();
        while let Ok(message) = socket.read() {
            if let tungstenite::Message::Text(text) = message {
                received.push(text.as_str().to_owned());
            }
        }
        received
    });
    let read = hql(&format!(
        "import net\ntap(websocket(\"ws://127.0.0.1:{port}\"), Json) | map(d => d.k)"
    ));
    assert_eq!(ok(&read), "1\n2\n");
    let written = hql(&format!(
        "import net\n[1, 2] | sink(websocket(\"ws://127.0.0.1:{port}\"), Json)"
    ));
    ok(&written);
    assert_eq!(server.join().unwrap(), ["1", "2"]);
}

#[test]
fn a_document_reaches_no_network() {
    let dir = tempdir().unwrap();
    // A document renders only in a vault.
    std::fs::write(dir.path().join("hql.toml"), "").unwrap();
    fs::write(
        dir.path().join("d.md"),
        "```hql\nimport net\nsource(http(\"http://127.0.0.1:1/\"), Json)\n```\n",
    )
    .unwrap();
    refused(
        &hql_in(dir.path(), &["render", "d.md"]),
        "nothing on the network",
    );
}
