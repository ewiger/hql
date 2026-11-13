//! A request answered by a headless agent session: `claude -p`.
//!
//! Everything the command is given comes from the request body, so the hash
//! that names a recording covers the whole invocation. The session runs in
//! the vault root, which is where its host files it, beside the user's own
//! conversations, and only the command's JSON result is read: the host's
//! session files are its own, and their format is not a contract.

use super::transport::Model;
use serde_json::{Value as J, json};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The request body for one invocation. A new session's id is derived from
/// the rest of the body, so it is known before the process starts and a
/// replayed run names the session the recorded one did.
pub(crate) fn request(model: &Model, system: &str, prompt: &str, resume: Option<&str>) -> J {
    let host = model.session.as_ref();
    let mut body = json!({
        "api": "claude-code",
        "model": model.model,
        "system": system,
        "prompt": prompt,
        "permission": host.map(|host| host.permission.clone()),
        "tools": host.map(|host| host.tools.clone()),
        "max_turns": host.map(|host| host.max_turns),
    });
    match resume {
        // A continuation forks: the parent's session may be open elsewhere,
        // and two writers interleave one transcript.
        Some(session) => {
            body["resume"] = json!(session);
        }
        None => {
            let id = uuid(&super::transport::key(&body));
            body["session"] = json!(id);
        }
    }
    body
}

/// A version 8 UUID from a hex digest: its first 128 bits, with the version
/// and variant set as RFC 9562 requires.
fn uuid(digest: &str) -> String {
    let bytes = Sha256::digest(digest.as_bytes());
    let mut raw: [u8; 16] = [0; 16];
    raw.copy_from_slice(&bytes[..16]);
    raw[6] = (raw[6] & 0x0f) | 0x80;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    let hex: String = raw.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The command line a body asks for, prompt excluded: it goes on stdin.
pub(crate) fn arguments(body: &J) -> Vec<String> {
    let text = |key: &str| body.get(key).and_then(J::as_str).unwrap_or_default();
    let mut arguments: Vec<String> = ["-p", "--output-format", "json"]
        .map(str::to_owned)
        .to_vec();
    if !text("system").is_empty() {
        arguments.extend([
            "--append-system-prompt".to_owned(),
            text("system").to_owned(),
        ]);
    }
    arguments.extend([
        "--permission-mode".to_owned(),
        text("permission").to_owned(),
    ]);
    let tools: Vec<&str> = body
        .get("tools")
        .and_then(J::as_array)
        .map(|tools| tools.iter().filter_map(J::as_str).collect())
        .unwrap_or_default();
    if !tools.is_empty() {
        arguments.extend(["--allowedTools".to_owned(), tools.join(",")]);
    }
    if let Some(turns) = body.get("max_turns").and_then(J::as_i64) {
        arguments.extend(["--max-turns".to_owned(), turns.to_string()]);
    }
    if !text("model").is_empty() {
        arguments.extend(["--model".to_owned(), text("model").to_owned()]);
    }
    match body.get("resume").and_then(J::as_str) {
        Some(session) => arguments.extend([
            "--resume".to_owned(),
            session.to_owned(),
            "--fork-session".to_owned(),
        ]),
        None => arguments.extend(["--session-id".to_owned(), text("session").to_owned()]),
    }
    arguments
}

/// Run one invocation in the vault root and read its JSON result.
///
/// # Errors
/// The command cannot start, exits without a JSON result, or reports an
/// error other than running out of turns.
pub(crate) fn invoke(root: &Path, model: &Model, body: &J) -> Result<J, String> {
    let host = model
        .session
        .as_ref()
        .ok_or("a claude-code entry names no session host")?;
    let prompt = body.get("prompt").and_then(J::as_str).unwrap_or_default();
    let mut child = Command::new(&host.command)
        .args(arguments(body))
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("`{}` cannot be started: {error}", host.command))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|error| format!("`{}`: {error}", host.command))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("`{}`: {error}", host.command))?;
    let result: J = serde_json::from_slice(&output.stdout).map_err(|_| {
        format!(
            "`{}` exited with {} and no JSON result: {}",
            host.command,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })?;
    let subtype = result
        .get("subtype")
        .and_then(J::as_str)
        .unwrap_or_default();
    if subtype != "success" && subtype != "error_max_turns" {
        return Err(format!(
            "the session ended with `{subtype}`: {}",
            result.get("result").and_then(J::as_str).unwrap_or_default()
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::ai::transport::{Api, SessionHost};

    fn model() -> Model {
        Model {
            api: Api::ClaudeCode,
            endpoint: String::new(),
            model: String::new(),
            api_key_env: None,
            max_tokens: 0,
            temperature: None,
            session: Some(SessionHost {
                command: "claude".to_owned(),
                permission: "plan".to_owned(),
                tools: vec!["Read".to_owned(), "Grep".to_owned()],
                max_turns: 4,
            }),
        }
    }

    #[test]
    fn a_new_session_is_named_before_it_starts_and_the_same_way_every_time() {
        let one = request(&model(), "Be brief.", "Which birds?", None);
        let two = request(&model(), "Be brief.", "Which birds?", None);
        assert_eq!(one["session"], two["session"]);
        let id = one["session"].as_str().unwrap();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "8");
        let arguments = arguments(&one);
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--session-id", id])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--allowedTools", "Read,Grep"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--permission-mode", "plan"])
        );
    }

    #[test]
    fn a_continuation_resumes_and_forks() {
        let body = request(&model(), "", "Why?", Some("abc"));
        let arguments = arguments(&body);
        assert!(
            arguments
                .windows(3)
                .any(|three| three == ["--resume", "abc", "--fork-session"])
        );
        assert!(!arguments.contains(&"--session-id".to_owned()));
        assert!(!arguments.contains(&"--append-system-prompt".to_owned()));
    }
}
