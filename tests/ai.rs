//! The `ai` extension: agents read from cards, runs kept as values, and
//! recordings that answer again without a network.
//!
//! A scripted model server on 127.0.0.1 stands in for a provider, under
//! `--ai record`; every later evaluation replays from what was recorded, with
//! the server gone.

use hql::extensions::ai::transport::Mode;
use hql::reporting::Mode as Report;
use hql::types::Value;
use hql::{run, vault};
use serde_json::{Value as J, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Answer each request with the next scripted body, recording what was sent.
fn serve(replies: Vec<J>) -> (u16, Arc<Mutex<Vec<J>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    std::thread::spawn(move || {
        for reply in replies {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            log.lock()
                .unwrap()
                .push(serde_json::from_slice(&body).unwrap());
            let text = reply.to_string();
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            )
            .unwrap();
        }
    });
    (port, seen)
}

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A vault with two agents, two evidence cards and a model on `port`.
fn vault_on(root: &Path, port: u16) -> vault::Vault {
    write(
        root,
        "hql.toml",
        &format!(
            "[ai]\nrecordings = \".hql/runs\"\n\n[ai.memory]\ndir = \"memory\"\n\n\
             [ai.models.default]\napi = \"messages\"\nendpoint = \"http://127.0.0.1:{port}/v1/messages\"\n\
             model = \"test-model\"\nmax_tokens = 256\ntemperature = 0\n"
        ),
    );
    write(
        root,
        "reviewer.md",
        "---\nagent:\n  tools: [count_named]\n  agents: [checker]\n  max_turns: 4\n---\n# Reviewer\n\nAnswer from the evidence alone.\n",
    );
    write(
        root,
        "checker.md",
        "---\nagent:\n  max_turns: 2\n---\n# Checker\n\nCheck one claim.\n",
    );
    write(root, "owl.md", "# Owl\n\nOwls hunt after dark.\n");
    write(root, "heron.md", "# Heron\n\nHerons wade.\n");
    write(root, "plain.md", "# Plain\n\nNo agent here.\n");
    vault::load(root).unwrap()
}

const PROGRAM: &str = r#"import ai
fn count_named(name: String) : Int = cards | filter(c => c.name == name) | size
reviewer = [[reviewer]] | Agent
ctx = cards | filter(c => c.name == "owl") | Context("evidence")
ctx | run(reviewer, tools = [Tool(count_named, "Count the cards with a name")])"#;

fn script() -> Vec<J> {
    vec![
        json!({"model": "test-model-1", "stop_reason": "tool_use", "usage": {"input_tokens": 10, "output_tokens": 2},
               "content": [{"type": "tool_use", "id": "c1", "name": "count_named", "input": {"name": "owl"}}]}),
        json!({"model": "test-model-1", "stop_reason": "tool_use", "usage": {"input_tokens": 12, "output_tokens": 2},
               "content": [{"type": "text", "text": "Checking."},
                           {"type": "tool_use", "id": "c2", "name": "checker", "input": {"task": "Is the owl nocturnal?"}}]}),
        json!({"model": "test-model-1", "stop_reason": "end_turn", "usage": {"input_tokens": 5, "output_tokens": 1},
               "content": [{"type": "text", "text": "Yes."}]}),
        json!({"model": "test-model-1", "stop_reason": "end_turn", "usage": {"input_tokens": 20, "output_tokens": 5},
               "content": [{"type": "text", "text": "There is one owl, and it hunts after dark."}]}),
    ]
}

fn value(source: &str, vault: &vault::Vault) -> Value {
    let outcome = run(source, vault, Report::Strict);
    outcome
        .value
        .unwrap_or_else(|| panic!("{source}: {:?}", outcome.reports))
}

fn failure(source: &str, vault: &vault::Vault) -> String {
    let outcome = run(source, vault, Report::Strict);
    assert!(outcome.value.is_none(), "{source}: expected a failure");
    format!("{:?}", outcome.reports)
}

fn field(source: &str, vault: &vault::Vault, path: &str) -> String {
    value(&format!("{source}\n{path}"), vault).to_string()
}

/// The program, run once against the scripted server and recorded.
fn recorded() -> (tempfile::TempDir, vault::Vault, Vec<J>) {
    let directory = tempfile::tempdir().unwrap();
    let (port, seen) = serve(script());
    let mut vault = vault_on(directory.path(), port);
    vault.ai.mode = Mode::Record;
    let review = format!("review = {}", PROGRAM.lines().last().unwrap());
    let program = PROGRAM.lines().take(4).collect::<Vec<_>>().join("\n");
    let first = field(&format!("{program}\n{review}"), &vault, "review.result");
    assert_eq!(first, "There is one owl, and it hunts after dark.");
    let sent = seen.lock().unwrap().clone();
    vault.ai.mode = Mode::Replay;
    (directory, vault, sent)
}

fn reviewed(path: &str, vault: &vault::Vault) -> String {
    let review = format!("review = {}", PROGRAM.lines().last().unwrap());
    let program = PROGRAM.lines().take(4).collect::<Vec<_>>().join("\n");
    field(&format!("{program}\n{review}"), vault, path)
}

#[test]
fn a_run_keeps_every_call_child_and_event_and_replays_without_a_network() {
    let (_directory, vault, sent) = recorded();
    // The first request: the agent's body as the system text, the rendered
    // prompt as the user turn, and the tool and the agent it may call.
    assert_eq!(sent.len(), 4);
    assert_eq!(
        sent[0]["system"],
        "# Reviewer\n\nAnswer from the evidence alone."
    );
    assert_eq!(
        sent[0]["messages"][0]["content"],
        "## evidence\n\nSource: owl\n\n# Owl\n\nOwls hunt after dark."
    );
    let tools: Vec<&str> = sent[0]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(tools, ["count_named", "checker"]);
    assert_eq!(
        sent[0]["tools"][0]["input_schema"]["required"],
        json!(["name"])
    );
    // The child is its own conversation, with the checker's instructions.
    assert_eq!(sent[2]["system"], "# Checker\n\nCheck one claim.");
    assert_eq!(sent[3]["messages"][4]["content"][0]["content"], "\"Yes.\"");

    // Replayed: the server is gone, and the run answers the same.
    assert_eq!(
        reviewed("review.result", &vault),
        "There is one owl, and it hunts after dark."
    );
    assert_eq!(reviewed("review.stop", &vault), "end");
    assert_eq!(
        reviewed("review.calls | map(c => c.status)", &vault),
        r#"["ok", "ok"]"#
    );
    assert_eq!(
        reviewed("review.calls | map(c => c.output)", &vault),
        r#"[1, "Yes."]"#
    );
    assert_eq!(
        reviewed("review.children | map(c => c.origin)", &vault),
        r#"["tool"]"#
    );
    assert_eq!(reviewed("review.turns | size", &vault), "6");
    assert_eq!(
        reviewed("review.usage", &vault),
        "{input_tokens: 42, output_tokens: 9}"
    );
    assert_eq!(reviewed("review.model.model", &vault), r#""test-model-1""#);
}

#[test]
fn trace_puts_a_childs_events_after_the_call_that_started_it() {
    let (_directory, vault, _) = recorded();
    assert_eq!(
        reviewed("review | trace | map(e => e.kind)", &vault),
        r#"["prompt", "request", "response", "tool_call", "request", "response", "tool_call", "prompt", "request", "response", "stop", "request", "response", "stop"]"#
    );
}

#[test]
fn a_continuation_not_recorded_fails_by_naming_the_mode_that_records() {
    let (_directory, vault, _) = recorded();
    let review = format!("review = {}", PROGRAM.lines().last().unwrap());
    let program = PROGRAM.lines().take(4).collect::<Vec<_>>().join("\n");
    let reports = failure(
        &format!("{program}\n{review}\nreview | ask(\"Why?\")"),
        &vault,
    );
    assert!(reports.contains("run with --ai record"), "{reports}");
}

#[test]
fn a_continuation_keeps_the_conversation_and_points_back() {
    let directory = tempfile::tempdir().unwrap();
    let mut replies = script();
    replies.push(json!({"model": "test-model-1", "stop_reason": "end_turn",
                        "content": [{"type": "text", "text": "Because the card says so."}]}));
    let (port, seen) = serve(replies);
    let mut vault = vault_on(directory.path(), port);
    vault.ai.mode = Mode::Record;
    let review = format!("review = {}", PROGRAM.lines().last().unwrap());
    let program = PROGRAM.lines().take(4).collect::<Vec<_>>().join("\n");
    let source = format!("{program}\n{review}\nwhy = review | ask(\"Why?\")");
    assert_eq!(
        field(&source, &vault, "why.result"),
        "Because the card says so."
    );
    assert_eq!(field(&source, &vault, "why.origin"), "ask");
    assert_eq!(
        field(&source, &vault, "why.parent.result"),
        "There is one owl, and it hunts after dark."
    );
    // The continuation sent the whole earlier conversation and one question.
    let sent = seen.lock().unwrap().clone();
    let last = sent.last().unwrap();
    let messages = last["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 7);
    assert_eq!(messages[6]["content"], "Why?");
}

#[test]
fn an_agent_is_a_card_that_says_it_is_one() {
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    assert_eq!(
        field("import ai", &vault, "([[reviewer]] | Agent).max_turns"),
        "4"
    );
    assert_eq!(
        field("import ai", &vault, "([[checker]] | Agent).model"),
        "default"
    );
    let reports = failure("[[plain]] | Agent", &vault);
    assert!(reports.contains("no `agent` table"), "{reports}");
}

#[test]
fn a_run_must_be_given_exactly_the_tools_its_card_lists() {
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    let reports = failure(
        "import ai\ncards | Context | run([[reviewer]] | Agent)",
        &vault,
    );
    assert!(
        reports.contains("lists the tool `count_named`"),
        "{reports}"
    );
}

#[test]
fn a_prompt_writes_its_instructions_then_each_label_in_order() {
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    let source = r#"owl = cards | filter(c => c.name == "owl") | Context("evidence")
notes = "Herons are not owls." | Context("memory")
p = concat(owl, notes) | Prompt("Which birds hunt at night?")"#;
    assert_eq!(
        field(source, &vault, "p.rendered"),
        "Which birds hunt at night?\n\n## evidence\n\nSource: owl\n\n# Owl\n\nOwls hunt after dark.\n\n## memory\n\nHerons are not owls."
    );
    assert_eq!(field(source, &vault, "concat(owl, notes) | size"), "2");
    assert_eq!(
        field(
            source,
            &vault,
            "concat(owl, notes) | filter(i => i.label == \"memory\") | map(i => i.text)"
        ),
        "[Herons are not owls.]"
    );
}

#[test]
fn a_tool_is_made_from_a_named_function_whose_parameters_a_schema_describes() {
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    let source = "fn pick(names: List<String>, limit: Option<Int>) : Int = size(names)";
    assert_eq!(
        field(source, &vault, "Tool(pick, \"Pick\").parameters"),
        r#"{"properties":{"limit":{"type":"integer"},"names":{"items":{"type":"string"},"type":"array"}},"required":["names"],"type":"object"}"#
    );
    let reports = failure("fn see(c: Card) : Int = 1\nTool(see, \"See\")", &vault);
    assert!(reports.contains("no JSON Schema"), "{reports}");
}

#[test]
fn memory_is_written_once_and_never_rewritten() {
    let directory = tempfile::tempdir().unwrap();
    let mut vault = vault_on(directory.path(), 9);
    let source = "import ai\n\"Owls hunt after dark.\" | remember(\"owls\")";
    // Replay writes nothing.
    assert!(failure(source, &vault).contains("--ai replay writes nothing"));
    vault.ai.mode = Mode::Record;
    assert_eq!(value(source, &vault).to_string(), "owls");
    assert_eq!(
        std::fs::read_to_string(directory.path().join("memory/owls.hmd")).unwrap(),
        "Owls hunt after dark.\n"
    );
    // The same content again is the same card; other content is refused.
    vault.ai.mode = Mode::Replay;
    assert_eq!(value(source, &vault).to_string(), "owls");
    vault.ai.mode = Mode::Record;
    let other = "import ai\n\"Owls sleep by day.\" | remember(\"owls\")";
    assert!(failure(other, &vault).contains("not rewritten"));
}

#[test]
fn checking_a_run_reaches_no_model() {
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    assert!(hql::check_in(PROGRAM, &vault).is_ok());
    assert!(!directory.path().join(".hql").exists());
}

#[test]
fn a_cell_reads_its_card_as_this_and_its_section_as_the_prompts_instructions() {
    use hql::transclude::{Emit, render_document};
    let directory = tempfile::tempdir().unwrap();
    let vault = vault_on(directory.path(), 9);
    let card = "---\ntags: [birds]\n---\n# Intro\n\nNot this.\n\n## Review\n\nWhich birds hunt after dark?\n\n```hql#eval id=title\nthis.title\n```\n\nAnswer from the evidence.\n\n```hql#eval id=p\nimport ai\ncards | filter(c => c.name == \"owl\") | Context(\"evidence\") | Prompt\n```\n";
    let rendered = render_document(
        card,
        Some(Path::new("review.hmd")),
        &vault,
        Report::Strict,
        Emit::After,
        None,
    )
    .unwrap();
    let outputs: Vec<&str> = rendered
        .cells
        .iter()
        .map(|cell| cell.markdown.as_str())
        .collect();
    assert_eq!(outputs[0], "Intro");
    // From the section's heading to the cell, without the other cell.
    assert_eq!(
        outputs[1],
        "## Review\n\nWhich birds hunt after dark?\n\nAnswer from the evidence.\n\n## evidence\n\nSource: owl\n\n# Owl\n\nOwls hunt after dark."
    );
    // Outside a cell there is no section, and the instructions are required.
    let reports = failure("import ai\ncards | Context | Prompt", &vault);
    assert!(
        reports.contains("outside a cell needs `instructions`"),
        "{reports}"
    );
}

fn render(card: &str, vault: &vault::Vault) -> hql::transclude::Rendered {
    hql::transclude::render_document(
        card,
        Some(Path::new("review.hmd")),
        vault,
        Report::Strict,
        hql::transclude::Emit::After,
        None,
    )
    .unwrap()
}

fn statuses(rendered: &hql::transclude::Rendered) -> Vec<&'static str> {
    rendered
        .cells
        .iter()
        .map(|cell| cell.status.label())
        .collect()
}

#[test]
fn under_host_a_request_waits_for_the_chat_and_its_answers_replay() {
    use hql::extensions::ai::transport;
    let directory = tempfile::tempdir().unwrap();
    let mut vault = vault_on(directory.path(), 9);
    vault.ai.mode = Mode::Host;
    let card = "# Review\n\nIs the owl nocturnal?\n\n```hql#eval id=answer\nimport ai\ncards | filter(c => c.name == \"owl\") | Context(\"evidence\") | run([[checker]] | Agent)\n```\n\n```hql#eval id=why\nimport ai\nanswer | ask(\"Why?\")\n```\n\n```hql#eval id=title\nthis.title\n```\n";
    // First pass: the first question waits, what reads it is skipped, and
    // what does not read it runs.
    let first = render(card, &vault);
    assert_eq!(statuses(&first), ["pending", "skipped", "ok"]);
    let waiting = transport::pending(directory.path(), &vault.ai);
    assert_eq!(waiting.len(), 1);
    assert_eq!(
        waiting[0].request["messages"][0]["content"],
        "# Review\n\nIs the owl nocturnal?\n\n## evidence\n\nSource: owl\n\n# Owl\n\nOwls hunt after dark."
    );
    assert_eq!(
        first.cells[0].pending.as_deref(),
        Some(waiting[0].key.as_str())
    );
    // A key nothing waits under, and an answer in the wrong shape, are refused.
    assert!(transport::answer(directory.path(), &vault.ai, "nothing", &json!({})).is_err());
    assert!(
        transport::answer(
            directory.path(),
            &vault.ai,
            &waiting[0].key,
            &json!({"text": "Yes."})
        )
        .is_err()
    );
    transport::answer(
        directory.path(),
        &vault.ai,
        &waiting[0].key,
        &transport::wrapped(&waiting[0].api, "Yes, it hunts after dark."),
    )
    .unwrap();
    // Second pass: the answer is a recording, and the follow-up now waits.
    let second = render(&first.document, &vault);
    assert_eq!(statuses(&second), ["ok", "pending", "ok"]);
    let waiting = transport::pending(directory.path(), &vault.ai);
    assert_eq!(waiting.len(), 1);
    transport::answer(
        directory.path(),
        &vault.ai,
        &waiting[0].key,
        &transport::wrapped(&waiting[0].api, "The card says so."),
    )
    .unwrap();
    let third = render(&second.document, &vault);
    assert_eq!(statuses(&third), ["ok", "ok", "ok"]);
    assert!(transport::pending(directory.path(), &vault.ai).is_empty());
    // Every answer is now a recording, so replay renders the same document.
    vault.ai.mode = Mode::Replay;
    assert_eq!(render(&third.document, &vault).document, third.document);
    let recorded = std::fs::read_dir(directory.path().join(".hql/runs"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .map(|entry| {
            serde_json::from_str::<J>(&std::fs::read_to_string(entry.path()).unwrap()).unwrap()
                ["answered_by"]
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(recorded, [json!("host"), json!("host")]);
}

/// A stand-in for `claude`: it records its arguments, working directory and
/// prompt, and prints a fixed result.
fn fake_claude(root: &Path, session: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = root.join("fake-claude.sh");
    let log = root.join("calls.log");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n{{ echo \"cwd=$(pwd)\"; echo \"args=$*\"; echo \"prompt=$(cat)\"; }} >> '{}'\n\
             printf '%s' '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Owls hunt at night.\",\"session_id\":\"{session}\",\"total_cost_usd\":0.01,\"num_turns\":2,\"usage\":{{\"input_tokens\":7}},\"modelUsage\":{{\"claude-test\":{{}}}}}}'\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn session_vault(root: &Path, permission: &str) -> vault::Vault {
    let script = fake_claude(root, "s-1");
    vault_on(root, 9);
    write(
        root,
        "hql.toml",
        &format!(
            "[ai.models.default]\napi = \"claude-code\"\ncommand = \"{}\"\npermission = \"{permission}\"\n\
             tools = [\"Read\", \"Grep\"]\nmax_turns = 3\n",
            script.display()
        ),
    );
    vault::load(root).unwrap()
}

#[test]
fn a_claude_code_session_runs_in_the_vault_and_a_continuation_forks_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut vault = session_vault(directory.path(), "plan");
    vault.ai.mode = Mode::Record;
    let source = "import ai\nreview = cards | filter(c => c.name == \"owl\") | Context(\"evidence\") | run([[checker]] | Agent)\nwhy = review | ask(\"Why?\")";
    assert_eq!(
        field(source, &vault, "review.result"),
        "Owls hunt at night."
    );
    assert_eq!(field(source, &vault, "review.model.session"), r#""s-1""#);
    assert_eq!(
        field(source, &vault, "review.model.model"),
        r#""claude-test""#
    );
    let log = std::fs::read_to_string(directory.path().join("calls.log")).unwrap();
    let root = directory.path().canonicalize().unwrap();
    assert!(log.contains(&format!("cwd={}", root.display())), "{log}");
    assert!(log.contains("--permission-mode plan"), "{log}");
    assert!(log.contains("--allowedTools Read,Grep"), "{log}");
    assert!(log.contains("--max-turns 3"), "{log}");
    assert!(log.contains("--append-system-prompt # Checker"), "{log}");
    assert!(log.contains("--session-id "), "{log}");
    assert!(log.contains("prompt=## evidence"), "{log}");
    // The continuation resumes the recorded session as a fork.
    assert_eq!(field(source, &vault, "why.origin"), "ask");
    let log = std::fs::read_to_string(directory.path().join("calls.log")).unwrap();
    assert!(log.contains("--resume s-1 --fork-session"), "{log}");
    assert!(log.contains("prompt=Why?"), "{log}");
    // Recorded: replay needs no command at all.
    std::fs::remove_file(directory.path().join("fake-claude.sh")).unwrap();
    vault.ai.mode = Mode::Replay;
    assert_eq!(field(source, &vault, "why.result"), "Owls hunt at night.");
}

#[test]
fn a_session_that_acts_without_approval_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let mut vault = session_vault(directory.path(), "bypassPermissions");
    vault.ai.mode = Mode::Record;
    let reports = failure(
        "import ai\ncards | Context | run([[checker]] | Agent)",
        &vault,
    );
    assert!(reports.contains("acts without approval"), "{reports}");
    let reports = failure(
        "import ai\nfn count_named(name: String) : Int = 1\ncards | Context | run([[reviewer]] | Agent, tools = [Tool(count_named, \"x\")])",
        &session_vault(directory.path(), "plan"),
    );
    assert!(reports.contains("offered no HQL tools"), "{reports}");
}
