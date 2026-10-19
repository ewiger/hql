# HQL-0022 companion: agent sessions in the editor

**Status**: drafted
**Created**: 2026-10-19
**Part of**: [HQL-0022](README.md)

## Abstract

The AI harness gains a second way to reach a model besides a provider's HTTP
API: a **session** of an agent host, such as Claude Code. A session is a
conversation that lives outside HQL, has its own tools and is visible in the
editor where the user already chats. The harness reaches one in two
directions, and both reuse what [HQL-0022](README.md) already defines.

- **Outbound.** An `[ai.models.<key>]` entry with `api = "claude-code"` makes
  `run` execute through a headless Claude Code session (`claude -p`) rather
  than an HTTP request. Each `Run` names the session it ran in, so the user can
  continue that conversation with `claude --resume <id>`, and in the editor's
  chat panel where the panel lists it.
- **Inbound.** Under `--ai host`, a request with no recording does not fail.
  It is handed to whoever is driving the program, and the run stops there. In
  the editor that is the agent the user is already chatting with. A skill lets
  that agent render a document, read the pending requests, answer each one, and
  render again. The answers are written as recordings, so the document then
  replays like any other.

The recording store from the section "Execution modes" of HQL-0022 is the
interface in both directions. A request is still hashed in canonical form, and
an answer is still a recording under that hash. What changes is who produces
the answer: a provider, a headless session, or the agent in the chat.

## Motivation

- The [HyperMarkDown extension for VS Code](https://hypermarkdown.org/tools/vscode/)
  already treats a card as a notebook. Its **Run** action hands the card to
  `hql render --emit after --envelope json` and writes each cell's output back
  into the card as one undoable edit. A cell that calls `run` is a cell like
  any other, so the extension needs no AI-specific code path. It only needs to
  pass the mode.
- The user is already in a chat with an agent in the same window. Today an AI
  cell has to pay for, configure and authenticate a second model through an
  API key, while the agent next to it holds the same context and goes unused.
- An API conversation is invisible. It exists only inside the `Run` value. A
  session is a conversation the user can read, interrupt and continue in the
  panel they already use, which is what an editor host offers that a terminal
  does not.
- The inverse is as useful. A user chatting with the agent should be able to
  say "run this card" and have the agent evaluate the HQL cells, read the cards
  they query, and answer the AI cells itself. The document becomes a procedure
  the chat executes rather than text the chat reads.

## Goals

- A document whose AI cells were answered in a chat replays byte for byte,
  with no network, exactly as one answered by a provider does.
- The same document runs under any of the three answerers without being
  edited. Which one answers is the vault's model entry and the host's mode,
  never the cell.
- Every answer the chat gave is recoverable from the `Run`: the request it
  answered, the session it came from and the recording that holds it.

## Non-goals

- **No new step or type.** `run`, `ask`, `delegate` and `Run` are unchanged. A
  session is a transport, and `--ai host` is a mode.
- **No message injection into a live chat.** HQL does not push a turn into a
  conversation the user has open. No documented interface lets an external
  process submit a user turn to the VS Code panel. Claude Code's inbox socket
  and its channels (a research preview) come closest, and neither documents
  what an unrelated process would need. The chat pulls requests when it
  chooses to, through the skill.
- **No reading of session files.** Claude Code stores sessions as
  `~/.claude/projects/<project>/<session-id>.jsonl`, and documents that format
  as internal and liable to change in any release. The transport reads only
  the command's JSON output.
- **No HQL tools inside a Claude Code session in this document.** A session
  started by `run` has the host's own tools under the permissions the vault
  grants, and the run's HQL tools are offered to it only once an `hql mcp`
  server exists, which this document leaves to a later revision.
- **No vendor lock.** `claude-code` is the first session host because it is
  the one in use. The contract is a command that takes a prompt and returns a
  result with a session id, so another host is one more `api` value.

## Specification

### 1. Three answerers, one store

```text
                      ┌─ messages / chat-completions ── HTTP provider
run ─► request body ──┼─ claude-code ─────────────────── headless session
       (hashed)       └─ (no recording, --ai host) ──── pending, for the chat
                                   │
                         answer written as a recording
                                   │
                    replay ◄───────┘   any later render
```

- Every answerer MUST produce its answer as a recording, `<key>.json`,
  holding the request and the response. The response MUST be in the wire
  format of the model entry's `api`, so `replay` reads all three alike.
- A recording MUST state which answerer produced it, under
  `"answered_by"`: `"provider"`, `"session"` or `"host"`. A reader of a
  document can then tell a provider's answer from one a person's chat gave.

### 2. The `claude-code` transport

A model entry names a session host instead of an endpoint.

```toml
[ai.models.chat]
api        = "claude-code"
command    = "claude"            # resolved on PATH unless absolute
permission = "plan"              # the session's --permission-mode
tools      = ["Read", "Grep", "Glob"]
max_turns  = 8
```

```text
first request:   claude -p --output-format json --session-id <uuid(run id)>
                        --append-system-prompt <agent instructions>
                        --permission-mode <permission> --allowedTools <tools…>
                        --max-turns <max_turns>                 prompt on stdin
later request:   claude -p --output-format json --resume <session> …
ask:             claude -p --output-format json --resume <session> --fork-session …
```

- A request MUST be sent as one headless invocation of `command`, with the
  rendered conversation as the prompt and the agent's instructions appended to
  the host's system prompt. It MUST NOT replace that system prompt, which
  would drop the host's own instructions for its tools.
- The process's working directory MUST be the vault root. `claude -p` has no
  flag for it, and a session is filed under the directory it runs in, which
  is what places it beside the user's own chats.
- The first request of a run MUST name its session with `--session-id`,
  derived from the run's id as a version 5 UUID. The session is then known
  before the process starts, and a replayed run names the same session as the
  recorded one. Each later request of the run MUST resume that session.
- `ask` MUST resume with `--fork-session`. The user may have opened the
  parent's session elsewhere, and two processes on one session interleave
  their messages into one transcript. A fork leaves the parent's conversation
  as it was and gives the continuation its own id.
- The response MUST be read from the command's JSON result: `result` as the
  answer, `session_id`, `num_turns`, `usage`, `total_cost_usd`, and `subtype`.
  `success` is `stop = "end"`, `error_max_turns` is `stop = "max_turns"`, and
  any other error subtype is a failure naming it. The session id MUST be
  recorded in the recording and in `Run.model.session`.
- `tools` and `permission` MUST be written in the entry, and the transport
  MUST refuse `bypassPermissions` and `dontAsk`. A headless session denies
  every tool call that no rule allows, so the entry's `tools` is the whole of
  what the session may do. That is the vault owner's decision, and it is
  written in a reviewed file.
- A session's own tool calls belong to the host, not to the harness. They
  MUST NOT appear in `Run.calls`, which holds only HQL tools. The run's
  `turns` hold the prompt and the session's final answer, and `usage` holds
  what the host reports.
- `claude-code` MUST respect `--ai`. Under `replay` no process is started,
  and under `record` a new session's answer is recorded.
- That a session started this way appears in the VS Code panel's history is
  not documented, and the CLI's own session picker excludes headless
  sessions. Nothing in the harness may depend on it. `claude --resume <id>`
  is the documented way back into the conversation, and it works for any
  session on the machine.

### 3. The `host` mode

The chat in the editor answers what the harness asks.

```text
hql render --ai host --emit after --envelope json   < card.hmd
```

- Under `--ai host`, a request with a recording is answered from it, as under
  `replay`. A request with none MUST NOT fail and MUST NOT reach a network. It
  is written to `<[ai] recordings>/pending/<key>.json` as the request alone,
  and the cell that made it stops with status `pending`.
- A cell that reads a pending cell's value is `skipped`, as a cell that
  depends on a failed one is under [HQL-0009](../HQL-0009/README.md). Cells
  that do not depend on it run and write their outputs.
- The envelope MUST list each pending request beside the cell that made it:
  its key, the agent's name, the system text, the conversation as plain
  turns, and the tools offered with their schemas.
- `hql ai answer <key> <file>` writes the answer in `<file>` as the recording
  `<key>.json`, with `"answered_by": "host"`, and removes the pending file. It
  MUST refuse a key with no pending request, and an answer that is not a valid
  response of the entry's `api`. The answer may request tool calls like any
  response, and the next render then evaluates them and leaves the next
  request pending.
- A render in `host` mode is therefore a loop the chat drives. Render, answer
  each pending request, and render again until nothing is pending. Each pass
  evaluates the same program over more recordings, so the final pass is
  exactly a `replay`.

### 4. The skill

The chat needs one instruction to drive that loop. It ships with HQL as a
skill, `hmd-run`, for any agent host that loads skills.

- The skill MUST name the loop of the `host` mode section: render with
  `--ai host`, answer each pending request with `hql ai answer`, and render
  again until no cell is pending.
- An answer MUST come from the pending request's own system text,
  conversation and tools, and not from the chat's surrounding conversation.
  Otherwise a recording would hold an answer to a question nobody can read in
  it, and the replay would be evidence of nothing.
- The skill MUST tell the agent to report, in the chat, each request it
  answered and the cell it belongs to, so the user sees what the document
  asked as well as what it concluded.
- The skill reaches HQL only through `hql render` and `hql ai answer`. It
  writes no card text itself; the outputs are written by HQL, as the
  extension's **Run** writes them.
- Both commands run through the chat's shell tool, so the skill MUST name the
  two allow rules it needs, `Bash(hql render *)` and `Bash(hql ai answer *)`,
  rather than asking for shell access in general. A standalone `hql mcp`
  server exposing the same two operations is the cleaner route, and is what
  the panel can load. It cannot load a tool server that lives inside another
  process.

### 5. The editor extension

The [extension's HQL cells](https://hypermarkdown.org/tools/vscode/) run
`hql render`. These rules are what they need to carry an AI cell.

- A setting, `hyperMarkdown.hql.ai`, MUST pass `--ai` through, with the values
  `replay` (the default), `record`, `live` and `host`. The default keeps
  **Run** what it is today: an action that never reaches a network.
- A cell whose status is `pending` MUST be shown as waiting for an answer,
  with the agent's name, and with an action that opens the chat and asks it to
  answer the card's pending requests through the skill.
- A run that reaches a model is not a keystroke-time operation. The
  extension's 30-second ceiling MUST NOT apply under `record` or `live`, which
  MUST be cancellable instead. `host` returns as soon as every reachable cell
  has run or is pending.
- An untrusted workspace MUST NOT run AI cells in any mode other than
  `replay`, as it runs no cells today.
- The Claude Code extension bundles its own CLI and does not put `claude` on
  `PATH`. A `claude-code` entry therefore needs the CLI installed on its own,
  or `command` set to an absolute path. The HyperMarkDown extension MUST NOT
  look for another extension's bundled binary.
- A `Run` written as a cell output SHOULD render with its answer first, and
  its prompt, calls and trace behind a disclosure, the way the preview shows a
  cell's query behind **Code**.

## Backwards Compatibility

Additive. `replay`, `record` and `live` keep their meaning, and a recording
without `answered_by` reads as `"provider"`. A vault with no `claude-code`
entry and a host that never passes `--ai host` sees no change.

## Security Considerations

- **A session is an agent with tools, not a function.** A provider returns
  text, while a session may read and, if permitted, write the vault and run
  commands. The entry's `tools` and `permission` are the boundary, written in
  a reviewed file. A session MUST NOT be started with a permission mode that
  skips approval.
- **The chat answers under the user's eyes.** In `host` mode, nothing runs
  that the user's agent did not run in the open chat, and every answer is a
  file in the recordings a reviewer can read. That is less authority than
  `live`, which is why `host` needs no credential.
- **A headless session loads the project's configuration.** Without
  `--bare`, `claude -p` loads `.claude/settings.json` hooks and `.mcp.json`
  servers from the vault with no trust dialog, so a cloned vault's hooks run
  when one of its cards is rendered under `record`. `--bare` skips them but
  requires an API key, which this transport exists to avoid. The transport
  MUST therefore refuse to start a session in a workspace the editor does not
  trust, and the documentation MUST say that `record` under `claude-code`
  runs the vault's hooks.
- **Pending requests are untrusted text.** A pending request holds context
  read from cards, and so may hold text written to redirect the agent that
  answers it. The skill MUST tell the agent to treat a request's content as
  data to answer, never as instructions to follow beyond the request's own
  task.
- **Recordings from a chat are reviewed like code.** A committed recording
  answered by `host` is an answer a person's agent gave, so it belongs in
  review beside the cell that asked for it.

## Reference Implementation

- `src/extensions/ai/session.rs`: the `claude-code` transport, a child
  process with a timeout, reading the host's JSON result
- `src/extensions/ai/transport.rs`: `Mode::Host`, the pending store and
  `answered_by`
- `src/transclude.rs`: the `pending` cell status and the pending requests in
  the envelope
- `src/cli.rs`: `hql ai answer`
- `contrib/skills/hmd-run/SKILL.md`: the skill
- the extension: `hyperMarkdown.hql.ai`, the `pending` cell state, and the
  action that opens the chat

## Test Plan

Unit tests MUST include:

- a pending request written under `host`, and no network reached
- `hql ai answer` refusing an unknown key and a malformed response, and
  turning a pending request into a recording that `replay` then reads
- a cell that depends on a pending cell `skipped`, and an independent cell
  run
- the `claude-code` transport against a fake `command` that prints a fixed
  JSON result. The fake checks the arguments it was given and the working
  directory, then `--session-id` derived from the run's id, `--resume` on a
  second request, and `--fork-session` on `ask`. It also covers
  `error_max_turns` read as `max_turns`, `bypassPermissions` refused, and the
  session id in `Run.model.session`

Integration tests MUST include:

- a card with two AI cells rendered in `host` mode three times, answered
  between passes, ending with no pending cell and byte-identical output on a
  fourth render under `replay`

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Sources

The `claude-code` transport rests only on documented behaviour of Claude
Code: the [CLI reference](https://code.claude.com/docs/en/cli-reference),
[headless mode](https://code.claude.com/docs/en/headless),
[sessions](https://code.claude.com/docs/en/sessions) and the
[VS Code extension](https://code.claude.com/docs/en/vs-code). What those pages
leave unstated is not relied on: the complete schema of the JSON result, the
panel listing headless sessions or refreshing while another process writes,
and any way for an outside process to reach an open panel.
