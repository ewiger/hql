//! A conversation in neither provider's shape, and its two wire formats.
//!
//! The harness keeps one representation of what was said, and writes it as a
//! request body only at the edge. Each format is a pure function of the
//! conversation, which is what makes a request's hash, and so its recording,
//! a property of the conversation rather than of the run that sent it.

use super::transport::{Api, Model};
use serde_json::{Value as J, json};

/// One message of a conversation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Message {
    /// What the program says.
    User(String),
    /// What the model answered, and the calls it requested.
    Assistant { text: String, calls: Vec<Call> },
    /// What the harness returned for each requested call, in request order.
    Results(Vec<Outcome>),
}

/// A call a response requests.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Call {
    pub id: String,
    pub tool: String,
    pub arguments: J,
}

/// What a call returned.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Outcome {
    pub id: String,
    pub output: String,
    pub failed: bool,
}

/// A tool as a model is told about it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: J,
}

/// Why a response ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stop {
    /// The model finished its answer.
    End,
    /// The model requested tool calls.
    Tools,
    /// The response reached `max_tokens`.
    MaxTokens,
    /// The provider declined to answer.
    Refused,
}

/// A response, read out of either format.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reply {
    pub text: String,
    pub calls: Vec<Call>,
    pub stop: Stop,
    /// The model id the response reports.
    pub model: String,
    pub usage: J,
}

/// The request body for a conversation.
pub(crate) fn request(model: &Model, system: &str, messages: &[Message], tools: &[ToolSpec]) -> J {
    let mut body = match model.api {
        Api::Messages => messages_request(model, system, messages, tools),
        // A model entry never names embeddings; the shape is the nearest one.
        Api::ChatCompletions | Api::Embeddings => chat_request(model, system, messages, tools),
    };
    if let (Some(temperature), J::Object(map)) = (model.temperature, &mut body) {
        map.insert("temperature".to_owned(), json!(temperature));
    }
    body
}

fn messages_request(model: &Model, system: &str, messages: &[Message], tools: &[ToolSpec]) -> J {
    let messages: Vec<J> = messages
        .iter()
        .map(|message| match message {
            Message::User(text) => json!({"role": "user", "content": text}),
            Message::Assistant { text, calls } => {
                let mut content = Vec::new();
                if !text.is_empty() {
                    content.push(json!({"type": "text", "text": text}));
                }
                for call in calls {
                    content.push(json!({
                        "type": "tool_use", "id": call.id, "name": call.tool, "input": call.arguments,
                    }));
                }
                json!({"role": "assistant", "content": content})
            }
            Message::Results(outcomes) => json!({
                "role": "user",
                "content": outcomes.iter().map(|outcome| json!({
                    "type": "tool_result",
                    "tool_use_id": outcome.id,
                    "content": outcome.output,
                    "is_error": outcome.failed,
                })).collect::<Vec<_>>(),
            }),
        })
        .collect();
    let mut body = json!({
        "model": model.model,
        "max_tokens": model.max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if !tools.is_empty() {
        body["tools"] = tools
            .iter()
            .map(|tool| {
                json!({"name": tool.name, "description": tool.description, "input_schema": tool.schema})
            })
            .collect();
    }
    body
}

fn chat_request(model: &Model, system: &str, messages: &[Message], tools: &[ToolSpec]) -> J {
    let mut written = Vec::new();
    if !system.is_empty() {
        written.push(json!({"role": "system", "content": system}));
    }
    for message in messages {
        match message {
            Message::User(text) => written.push(json!({"role": "user", "content": text})),
            Message::Assistant { text, calls } => {
                let mut turn = json!({"role": "assistant", "content": text});
                if !calls.is_empty() {
                    turn["tool_calls"] = calls
                        .iter()
                        .map(|call| {
                            json!({
                                "id": call.id,
                                "type": "function",
                                "function": {"name": call.tool, "arguments": call.arguments.to_string()},
                            })
                        })
                        .collect();
                }
                written.push(turn);
            }
            Message::Results(outcomes) => {
                for outcome in outcomes {
                    written.push(json!({
                        "role": "tool", "tool_call_id": outcome.id, "content": outcome.output,
                    }));
                }
            }
        }
    }
    let mut body = json!({
        "model": model.model,
        "max_tokens": model.max_tokens,
        "messages": written,
    });
    if !tools.is_empty() {
        body["tools"] = tools
            .iter()
            .map(|tool| {
                json!({"type": "function", "function": {
                    "name": tool.name, "description": tool.description, "parameters": tool.schema,
                }})
            })
            .collect();
    }
    body
}

/// Read a response body in a model's format.
///
/// # Errors
/// Says what the body is missing when it is not a response of that format.
pub(crate) fn reply(api: Api, response: &J) -> Result<Reply, String> {
    match api {
        Api::Messages => messages_reply(response),
        Api::ChatCompletions => chat_reply(response),
        Api::Embeddings => Err("an embeddings endpoint answers no conversation".to_owned()),
    }
}

fn messages_reply(response: &J) -> Result<Reply, String> {
    let content = response
        .get("content")
        .and_then(J::as_array)
        .ok_or("a Messages response holds `content`")?;
    let mut text = String::new();
    let mut calls = Vec::new();
    for block in content {
        match block.get("type").and_then(J::as_str) {
            Some("text") => text.push_str(block.get("text").and_then(J::as_str).unwrap_or("")),
            Some("tool_use") => calls.push(Call {
                id: string(block, "id"),
                tool: string(block, "name"),
                arguments: block.get("input").cloned().unwrap_or(json!({})),
            }),
            _ => {}
        }
    }
    let stop = match response.get("stop_reason").and_then(J::as_str) {
        Some("tool_use") => Stop::Tools,
        Some("max_tokens") => Stop::MaxTokens,
        Some("refusal") => Stop::Refused,
        _ if !calls.is_empty() => Stop::Tools,
        _ => Stop::End,
    };
    Ok(Reply {
        text,
        calls,
        stop,
        model: string(response, "model"),
        usage: response.get("usage").cloned().unwrap_or(json!({})),
    })
}

fn chat_reply(response: &J) -> Result<Reply, String> {
    let choice = response
        .get("choices")
        .and_then(J::as_array)
        .and_then(|choices| choices.first())
        .ok_or("a chat-completions response holds `choices`")?;
    let message = choice.get("message").ok_or("a choice holds `message`")?;
    let calls = message
        .get("tool_calls")
        .and_then(J::as_array)
        .map(|calls| {
            calls
                .iter()
                .map(|call| {
                    let function = call.get("function").cloned().unwrap_or(json!({}));
                    let arguments = function
                        .get("arguments")
                        .and_then(J::as_str)
                        .and_then(|text| serde_json::from_str(text).ok())
                        .unwrap_or(json!({}));
                    Call {
                        id: string(call, "id"),
                        tool: string(&function, "name"),
                        arguments,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let stop = match choice.get("finish_reason").and_then(J::as_str) {
        Some("tool_calls") => Stop::Tools,
        Some("length") => Stop::MaxTokens,
        Some("content_filter") => Stop::Refused,
        _ if !calls.is_empty() => Stop::Tools,
        _ => Stop::End,
    };
    Ok(Reply {
        text: string(message, "content"),
        calls,
        stop,
        model: string(response, "model"),
        usage: response.get("usage").cloned().unwrap_or(json!({})),
    })
}

fn string(value: &J, key: &str) -> String {
    value
        .get(key)
        .and_then(J::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(api: Api) -> Model {
        Model {
            api,
            endpoint: String::new(),
            model: "m".to_owned(),
            api_key_env: None,
            max_tokens: 64,
            temperature: Some(0.0),
        }
    }

    fn conversation() -> Vec<Message> {
        vec![
            Message::User("find owls".to_owned()),
            Message::Assistant {
                text: String::new(),
                calls: vec![Call {
                    id: "c1".to_owned(),
                    tool: "find".to_owned(),
                    arguments: json!({"topic": "owls"}),
                }],
            },
            Message::Results(vec![Outcome {
                id: "c1".to_owned(),
                output: "[]".to_owned(),
                failed: false,
            }]),
        ]
    }

    #[test]
    fn a_messages_request_carries_the_system_text_tools_and_results() {
        let tools = [ToolSpec {
            name: "find".to_owned(),
            description: "Find".to_owned(),
            schema: json!({"type": "object"}),
        }];
        let body = request(&model(Api::Messages), "Be brief.", &conversation(), &tools);
        assert_eq!(body["system"], "Be brief.");
        assert_eq!(body["temperature"], 0.0);
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_use");
        assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "c1");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
    }

    #[test]
    fn a_chat_request_writes_the_system_text_as_a_message_and_arguments_as_text() {
        let body = request(
            &model(Api::ChatCompletions),
            "Be brief.",
            &conversation(),
            &[],
        );
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(
            body["messages"][2]["tool_calls"][0]["function"]["arguments"],
            r#"{"topic":"owls"}"#
        );
        assert_eq!(body["messages"][3]["role"], "tool");
    }

    #[test]
    fn each_format_reads_back_its_calls_and_why_it_stopped() {
        let messages = json!({
            "model": "m-1", "stop_reason": "tool_use", "usage": {"input_tokens": 3},
            "content": [{"type": "text", "text": "Looking."},
                        {"type": "tool_use", "id": "c1", "name": "find", "input": {"topic": "owls"}}],
        });
        let read = reply(Api::Messages, &messages).unwrap();
        assert_eq!(
            (read.stop, read.text.as_str(), read.calls.len()),
            (Stop::Tools, "Looking.", 1)
        );
        let chat = json!({
            "model": "m-2",
            "choices": [{"finish_reason": "stop", "message": {"content": "Owls."}}],
        });
        let read = reply(Api::ChatCompletions, &chat).unwrap();
        assert_eq!(
            (read.stop, read.text.as_str(), read.model.as_str()),
            (Stop::End, "Owls.", "m-2")
        );
        let refused = json!({"model": "m", "stop_reason": "refusal", "content": []});
        assert_eq!(reply(Api::Messages, &refused).unwrap().stop, Stop::Refused);
    }
}
