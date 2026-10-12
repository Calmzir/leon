//! opencode, spoken to both ways.
//!
//! opencode serves the Agent Client Protocol on its standard streams (checked
//! against opencode 2.0.24):
//!
//! ```text
//! opencode acp
//! ```
//!
//! Each line either way is one JSON-RPC 2.0 value; the `jsonrpc` member is
//! required. The phone writes:
//!
//! * `initialize`, then `session/new` (with the folder to work in, which
//!   must be a whole path) or `session/resume` (an id any opencode gave a
//!   conversation; unlike `session/load` it does not print the conversation
//!   again). The two are written at once: they are answered in order;
//! * `session/set_config_option` for the model, the effort and the mode that
//!   were asked for, one after the other, each once the one before was
//!   answered. Requests are served as they come, not in order, so nothing
//!   that needs the session is written before the session's answer. The
//!   models of every provider are only known a moment after that: the first
//!   `config_option_update` says so, and a model or an effort waits for it
//!   (set before, the session would be tied to a model it may not be able
//!   to use);
//! * `session/prompt` with the prompt. Its answer is the end of the turn;
//! * `session/cancel` to stop it;
//! * the answer to what the agent asks.
//!
//! The agent prints `session/update` notifications (`agent_message_chunk`,
//! `agent_thought_chunk`, `tool_call`, `tool_call_update`, `plan`,
//! `usage_update`, `config_option_update`…) and asks with
//! `session/request_permission`, which lists what it may be answered with
//! (`allow_once`, `allow_always`, `reject_once`). A refusal ends the turn:
//! that is opencode's way. It cannot be told more while it works (a second
//! prompt is refused), and it reads no file and runs no terminal through the
//! phone: this client offers neither.
//!
//! opencode has no mode in which it asks nothing. [`MODE_AUTO`] is this
//! library's: the default agent, with every permission it asks for allowed
//! at once, as `opencode --auto` does.
//!
//! The server ends when its input does: after the prompt's answer, its input
//! is closed.

use std::collections::{BTreeMap, HashSet, VecDeque};

use serde_json::{json, Value};

use crate::agents::{strings, Action, AgentSession, Outgoing, Step, TurnSpec};
use crate::bridge::{id_tag, tag_id};
use crate::claude::{AgentMode, AgentModel, AgentOptions};
use crate::error::MobileError;
use crate::events::{
    blocks, cap_detail, clip, edit_detail, first_line, input_detail, input_summary, text_detail,
    text_of, written_detail, Answer, PermissionChoice, PermissionEffect, QuestionKind, TurnEvent,
};

/// The ids of this client's own requests. They are fixed, so that a turn
/// read again knows which answer is which.
const ID_HELLO: &str = "leon-init";
const ID_SESSION: &str = "leon-session";
const ID_MODEL: &str = "leon-model";
const ID_EFFORT: &str = "leon-effort";
const ID_MODE: &str = "leon-mode";
const ID_TURN: &str = "leon-turn";
/// A request a process that was never told the prompt writes in place of
/// `session/prompt`: its answer means nobody had started the turn.
const ID_LOST: &str = "leon-lost";

const TAG_HELLO: &str = "h.init";
const TAG_SESSION: &str = "h.session";
const TAG_TURN: &str = "h.turn";

/// The mode of this library's making: the default agent, never asking.
pub(crate) const MODE_AUTO: &str = "leon-auto";
/// The agent [`MODE_AUTO`] works as.
const AUTO_AGENT: &str = "build";

/// How many characters of a change are shown to whoever must approve it.
const DETAIL_CHARS: usize = 20_000;
/// How many lines that were not the server's are kept to explain a failure.
const NOISE_LINES: usize = 6;

/// What is said when the turn was never started.
const NEVER_STARTED: &str =
    "The prompt did not reach opencode before the connection was lost. Send it again.";

/// The command line of a turn. What the turn is about travels in requests.
pub(crate) fn argv(turn: &TurnSpec<'_>) -> Vec<String> {
    strings(&[turn.program, "acp"])
}

/// A session for one turn.
pub(crate) fn session(unique: u64) -> Box<dyn AgentSession> {
    Box::new(OpencodeSession {
        unique,
        prompt: None,
        resumed: false,
        auto: false,
        todo: VecDeque::new(),
        waits_for_models: false,
        models_known: false,
        setting: false,
        session: None,
        started: false,
        ready: false,
        prompt_queued: false,
        prompt_delivered: false,
        finished: false,
        idle: false,
        stop_wanted: false,
        asked: BTreeMap::new(),
        tools: BTreeMap::new(),
        announced: HashSet::new(),
        message: None,
        draft: String::new(),
        mode: None,
        model: None,
        cost: None,
        noise: VecDeque::new(),
    })
}

fn request(id: &str, method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

/// The opening request, as a line. No capability is offered: the agent
/// reads files and runs commands itself, on its own computer.
fn hello() -> String {
    request(
        ID_HELLO,
        "initialize",
        json!({
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "leon-mobile",
                "title": "Leon",
                "version": env!("CARGO_PKG_VERSION"),
            },
        }),
    )
}

/// One option of the session still to be set: the request's id, the
/// option, its value.
#[derive(Debug)]
struct Setting {
    id: &'static str,
    option: &'static str,
    value: String,
}

/// What the agent asked and has no answer to yet.
#[derive(Debug)]
struct Asked {
    /// The request's id as the agent wrote it (a number, or text).
    id: Value,
    /// What it may be answered with: the option's id and its kind.
    options: Vec<(String, String)>,
}

/// A tool the agent uses, as far as it said.
#[derive(Debug, Default)]
struct Tool {
    /// The tool's own name (`shell`, `write`), from its first mention.
    name: String,
    /// Whether what it does was shown in full when it started.
    shown: bool,
}

struct OpencodeSession {
    /// Makes the ids of this process's own requests unlike any other's.
    unique: u64,
    /// What the person wrote; nothing in a process that found the turn
    /// running.
    prompt: Option<String>,
    /// The conversation is an earlier one: what it has cost so far is not
    /// known, and so not what this turn costs.
    resumed: bool,
    /// Every permission asked for is allowed at once ([`MODE_AUTO`]).
    auto: bool,
    /// What is still to be set before the prompt, in order.
    todo: VecDeque<Setting>,
    /// A model or an effort is to be set, which needs the models known.
    waits_for_models: bool,
    models_known: bool,
    /// An option was written and its answer is awaited.
    setting: bool,
    session: Option<String>,
    started: bool,
    /// The session answered: what needs it may be written.
    ready: bool,
    /// The prompt (or what stands for it) was handed over to be written.
    prompt_queued: bool,
    /// The prompt reached the agent (this process's, or an earlier one's).
    prompt_delivered: bool,
    /// The turn ended (or never began): nothing more can be said in it.
    finished: bool,
    idle: bool,
    /// An interrupt was asked for before the session had an id.
    stop_wanted: bool,
    /// Open questions, by the text of the request's id.
    asked: BTreeMap<String, Asked>,
    tools: BTreeMap<String, Tool>,
    /// Tools shown as in use and not as done.
    announced: HashSet<String>,
    /// The message being written: its id, and what it says so far.
    message: Option<String>,
    draft: String,
    mode: Option<String>,
    model: Option<String>,
    /// What the conversation has cost, as last said.
    cost: Option<f64>,
    noise: VecDeque<String>,
}

impl OpencodeSession {
    fn next_id(&mut self, kind: &str) -> String {
        self.unique += 1;
        format!("leon-{kind}-{}", self.unique)
    }

    fn resolve(&mut self, id: &str, step: &mut Step) {
        if self.asked.remove(id).is_some() {
            step.events
                .push(TurnEvent::QuestionResolved { id: id.to_owned() });
        }
    }

    fn resolve_all(&mut self, step: &mut Step) {
        let open: Vec<String> = self.asked.keys().cloned().collect();
        for id in open {
            self.resolve(&id, step);
        }
    }

    /// The conversation has this id: said once.
    fn session_is(&mut self, id: &str, step: &mut Step) {
        if id.is_empty() {
            return;
        }
        if self.session.is_none() {
            self.session = Some(id.to_owned());
        }
        if !self.started {
            self.started = true;
            step.events.push(TurnEvent::Started {
                session_id: id.to_owned(),
                model: self.model.clone(),
            });
        }
        if self.stop_wanted {
            self.stop_wanted = false;
            step.send.extend(self.cancel());
        }
    }

    /// The session's options as the agent lists them: what it is set to.
    fn options_are(&mut self, options: &Value, step: &mut Step) {
        for option in blocks(options) {
            let Some(current) = option["currentValue"].as_str() else {
                continue;
            };
            match option["id"].as_str() {
                Some("model") => self.model = Some(current.to_owned()),
                Some("mode") => self.mode_is(current, step),
                _ => {}
            }
        }
    }

    fn mode_is(&mut self, mode: &str, step: &mut Step) {
        // Its agent under this library's mode is still that mode.
        let mode = if self.auto && mode == AUTO_AGENT {
            MODE_AUTO
        } else {
            mode
        };
        if !mode.is_empty() && self.mode.as_deref() != Some(mode) {
            self.mode = Some(mode.to_owned());
            step.events.push(TurnEvent::ModeChanged {
                mode: mode.to_owned(),
            });
        }
    }

    fn set(&self, id: &str, option: &str, value: &str) -> Option<String> {
        let session = self.session.as_deref()?;
        Some(request(
            id,
            "session/set_config_option",
            json!({ "sessionId": session, "configId": option, "value": value }),
        ))
    }

    /// The next thing to write before the turn runs: the next option, or
    /// the prompt.
    fn advance(&mut self, step: &mut Step) {
        if !self.ready
            || self.finished
            || self.prompt_queued
            || self.setting
            || (self.waits_for_models && !self.models_known)
        {
            return;
        }
        if let Some(setting) = self.todo.pop_front() {
            if let Some(line) = self.set(setting.id, setting.option, &setting.value) {
                self.setting = true;
                step.send.push(Outgoing {
                    tag: format!("h.{}", setting.option),
                    line,
                });
            }
            return;
        }
        let line = match (&self.prompt, &self.session) {
            (Some(prompt), Some(session)) => request(
                ID_TURN,
                "session/prompt",
                json!({ "sessionId": session, "prompt": [{ "type": "text", "text": prompt }] }),
            ),
            (Some(_), None) => return,
            // This process found the turn running and was never told the
            // prompt. Whoever started it wrote the prompt under this tag: if
            // that arrived, this line is never written; if it did not, the
            // answer to this one (no such method) says so.
            (None, _) => request(ID_LOST, "leon/lost", json!({})),
        };
        self.prompt_queued = true;
        step.send.push(Outgoing {
            tag: TAG_TURN.to_owned(),
            line,
        });
    }

    /// The turn ended without a result.
    fn fail(&mut self, error: String, step: &mut Step) {
        if self.finished {
            return;
        }
        self.end(step);
        step.events.push(TurnEvent::Finished {
            ok: false,
            error: Some(error),
            cost_usd: None,
            duration_ms: None,
            denied_tools: Vec::new(),
            interrupted: false,
        });
    }

    /// What every end of a turn does first.
    fn end(&mut self, step: &mut Step) {
        self.settle(step);
        self.finished = true;
        self.idle = true;
        self.resolve_all(step);
        // What a stopped turn was doing is never said to end.
        let mut running: Vec<String> = self.announced.drain().collect();
        running.sort();
        for id in running {
            step.events.push(TurnEvent::ToolResult {
                id,
                ok: false,
                summary: "stopped".to_owned(),
                detail: String::new(),
            });
        }
    }

    /// What stops the turn: the notification, and "cancelled" to whatever
    /// the agent still asks, as the protocol wants.
    fn cancel(&mut self) -> Vec<Outgoing> {
        let Some(session) = self.session.clone() else {
            return Vec::new();
        };
        let id = self.next_id("stop");
        let mut out = vec![Outgoing {
            tag: format!("i.{id}"),
            line: json!({
                "jsonrpc": "2.0",
                "method": "session/cancel",
                "params": { "sessionId": session },
            })
            .to_string(),
        }];
        for (id, asked) in &self.asked {
            out.push(Outgoing {
                tag: id_tag('a', id),
                line: outcome(&asked.id, json!({ "outcome": "cancelled" })),
            });
        }
        out
    }

    fn answered(&mut self, value: &Value, step: &mut Step) {
        let failed = value["error"]["message"]
            .as_str()
            .map(|text| {
                text.strip_prefix("Internal error: ")
                    .unwrap_or(text)
                    .to_owned()
            })
            .or_else(|| (!value["error"].is_null()).then(|| value["error"].to_string()));
        match value["id"].as_str().unwrap_or_default() {
            ID_HELLO => {
                if let Some(error) = failed {
                    self.fail(error, step);
                }
            }
            ID_SESSION => {
                if let Some(error) = failed {
                    self.fail(error, step);
                    return;
                }
                let result = &value["result"];
                self.model = blocks(&result["configOptions"])
                    .iter()
                    .find(|option| option["id"] == "model")
                    .and_then(|option| option["currentValue"].as_str())
                    .map(str::to_owned);
                // A resumed session is not named again: its id is the one
                // that was asked for.
                let id = result["sessionId"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| self.session.clone());
                match id {
                    Some(id) => self.session_is(&id, step),
                    None if self.prompt.is_some() => {
                        self.fail("opencode did not name the conversation".to_owned(), step);
                        return;
                    }
                    // Found running: an update will name it.
                    None => {}
                }
                self.options_are(&result["configOptions"], step);
                self.ready = true;
                self.advance(step);
            }
            // An option it does not have (a model that is gone, an effort
            // this model lacks) is no reason not to run: the turn goes on
            // with what that computer has.
            ID_MODEL | ID_EFFORT | ID_MODE => {
                self.setting = false;
                if failed.is_none() {
                    self.options_are(&value["result"]["configOptions"], step);
                }
                self.advance(step);
            }
            ID_TURN => {
                if self.finished {
                    return;
                }
                if let Some(error) = failed {
                    self.fail(error, step);
                    return;
                }
                self.end(step);
                let reason = value["result"]["stopReason"].as_str().unwrap_or("end_turn");
                let ok = reason == "end_turn";
                let interrupted = reason == "cancelled";
                let error = (!ok && !interrupted).then(|| match reason {
                    "max_tokens" => "It ran out of room for its answer.".to_owned(),
                    "max_turn_requests" => "It reached its limit of steps for one turn.".to_owned(),
                    "refusal" => "The model declined to go on.".to_owned(),
                    other => format!("The turn stopped: {other}"),
                });
                step.events.push(TurnEvent::Finished {
                    ok,
                    error,
                    // The cost said is the conversation's: this turn's only
                    // when the conversation began with it.
                    cost_usd: self.cost.filter(|_| !self.resumed && self.prompt.is_some()),
                    duration_ms: None,
                    denied_tools: Vec::new(),
                    interrupted,
                });
            }
            ID_LOST if !self.prompt_delivered => self.fail(NEVER_STARTED.to_owned(), step),
            _ => {}
        }
    }

    fn ask(&mut self, value: &Value, step: &mut Step) {
        let id = id_text(&value["id"]);
        let params = &value["params"];
        if value["method"] != "session/request_permission" {
            // Files and terminals of a client this is not: said at once, so
            // the agent does not wait for nobody.
            step.send.push(Outgoing {
                tag: id_tag('r', &id),
                line: json!({
                    "jsonrpc": "2.0",
                    "id": value["id"],
                    "error": { "code": -32601, "message": "not supported by this client" },
                })
                .to_string(),
            });
            return;
        }
        if let Some(session) = params["sessionId"].as_str() {
            self.session_is(session, step);
        }
        let options: Vec<(String, String)> = blocks(&params["options"])
            .iter()
            .map(|option| (text_of(&option["optionId"]), text_of(&option["kind"])))
            .collect();
        if self.auto {
            let allow = options
                .iter()
                .find(|option| option.1 == "allow_once")
                .or_else(|| options.iter().find(|option| option.1 == "allow_always"));
            if let Some((option, _)) = allow {
                step.send.push(Outgoing {
                    tag: id_tag('a', &id),
                    line: outcome(
                        &value["id"],
                        json!({ "outcome": "selected", "optionId": option }),
                    ),
                });
                return;
            }
        }
        let call = &params["toolCall"];
        let tool = match call["kind"].as_str() {
            Some("execute") => "shell".to_owned(),
            Some(kind) if !kind.is_empty() && kind != "other" => kind.to_owned(),
            _ => self
                .tools
                .get(call["toolCallId"].as_str().unwrap_or_default())
                .map(|tool| tool.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "tool".to_owned()),
        };
        let choices = blocks(&params["options"])
            .iter()
            .filter_map(|option| {
                let name = text_of(&option["name"]);
                let (effect, label) = match option["kind"].as_str()? {
                    "allow_once" => (PermissionEffect::AllowOnce, name),
                    "allow_always" => (PermissionEffect::AllowAlways, name),
                    // What opencode does with a refusal: the turn is over.
                    "reject_once" => (PermissionEffect::Deny, format!("{name}, and stop the turn")),
                    "reject_always" => (PermissionEffect::Deny, name),
                    _ => return None,
                };
                Some(PermissionChoice {
                    id: text_of(&option["optionId"]),
                    effect,
                    label,
                    scope: None,
                    takes_reason: false,
                })
            })
            .collect();
        // Asked twice (a replay after a gap): the later asking stands.
        self.asked.insert(
            id.clone(),
            Asked {
                id: value["id"].clone(),
                options,
            },
        );
        step.events.push(TurnEvent::Question {
            id,
            kind: QuestionKind::Permission {
                tool,
                summary: first_line(&text_of(&call["title"])),
                detail: detail_of(call),
                reason: None,
                choices,
            },
        });
    }

    fn answer(&self, id: &str, answer: Answer) -> Result<String, MobileError> {
        let asked = self.asked.get(id).ok_or(MobileError::UnknownQuestion)?;
        let Answer::Permission { choice_id, .. } = answer else {
            return Err(MobileError::BadAnswer);
        };
        let known = |wanted: &str| asked.options.iter().any(|option| option.0 == wanted);
        let result = if known(&choice_id) {
            json!({ "outcome": "selected", "optionId": choice_id })
        } else if choice_id == "deny" {
            // A refusal that names no option: the agent's own, if it has one.
            match asked
                .options
                .iter()
                .find(|option| option.1.starts_with("reject"))
            {
                Some((option, _)) => json!({ "outcome": "selected", "optionId": option }),
                None => json!({ "outcome": "cancelled" }),
            }
        } else {
            return Err(MobileError::BadAnswer);
        };
        Ok(outcome(&asked.id, result))
    }

    /// The message being written is whole: said as one, so that the next
    /// one starts on its own.
    fn settle(&mut self, step: &mut Step) {
        self.message = None;
        let text = std::mem::take(&mut self.draft);
        if !text.is_empty() {
            step.events.push(TurnEvent::Text { text });
        }
    }

    fn tool(&mut self, update: &Value, step: &mut Step) {
        let id = text_of(&update["toolCallId"]);
        if id.is_empty() {
            return;
        }
        let title = text_of(&update["title"]);
        let tool = self.tools.entry(id.clone()).or_default();
        // The first mention names the tool; later ones say what it does.
        if tool.name.is_empty() {
            tool.name = match update["kind"].as_str() {
                Some("execute") => "shell".to_owned(),
                _ if !title.is_empty() => first_line(&title),
                Some(kind) => kind.to_owned(),
                None => "tool".to_owned(),
            };
        }
        let status = update["status"].as_str().unwrap_or("pending");
        if status == "pending" && update["sessionUpdate"] == "tool_call" {
            // Nothing is known of it yet but that it will come.
            return;
        }
        let name = tool.name.clone();
        let done = matches!(status, "completed" | "failed");
        let changes = changes_detail(update);
        if self.announced.insert(id.clone()) {
            let summary = match input_summary(&update["rawInput"]) {
                summary if summary.is_empty() && title != name => first_line(&title),
                summary => summary,
            };
            let detail = if changes.is_empty() {
                input_detail(&update["rawInput"], &summary)
            } else {
                changes.clone()
            };
            tool.shown = !detail.is_empty();
            self.settle(step);
            step.events.push(TurnEvent::ToolUse {
                id: id.clone(),
                name,
                summary,
                detail,
            });
        }
        if done {
            self.announced.remove(&id);
            let shown = self.tools.remove(&id).is_some_and(|tool| tool.shown);
            let texts: Vec<&str> = blocks(&update["content"])
                .iter()
                .filter_map(|block| block["content"]["text"].as_str())
                .collect();
            let said = texts
                .first()
                .copied()
                .or(update["rawOutput"]["error"].as_str())
                .unwrap_or_default();
            let summary = first_line(said);
            let whole = match texts.is_empty() {
                true => update["rawOutput"]["output"]
                    .as_str()
                    .unwrap_or(said)
                    .to_owned(),
                false => texts.join("\n"),
            };
            let mut detail = text_detail(&whole, &summary);
            // A change only said once it was made is not lost.
            if detail.is_empty() && !shown {
                detail = changes;
            }
            step.events.push(TurnEvent::ToolResult {
                id,
                ok: status == "completed",
                summary,
                detail,
            });
        }
    }

    fn told(&mut self, value: &Value, step: &mut Step) {
        if value["method"] != "session/update" {
            return;
        }
        let params = &value["params"];
        if let Some(session) = params["sessionId"].as_str() {
            self.session_is(session, step);
        }
        let update = &params["update"];
        match update["sessionUpdate"].as_str().unwrap_or_default() {
            "agent_message_chunk" => {
                let Some(text) = update["content"]["text"].as_str().filter(|t| !t.is_empty())
                else {
                    return;
                };
                let id = update["messageId"].as_str();
                if self.message.is_some() && self.message.as_deref() != id {
                    self.settle(step);
                }
                self.message = id.map(str::to_owned);
                self.draft.push_str(text);
                step.events.push(TurnEvent::TextDelta {
                    text: text.to_owned(),
                });
            }
            "agent_thought_chunk" => {
                if let Some(text) = update["content"]["text"].as_str().filter(|t| !t.is_empty()) {
                    step.events.push(TurnEvent::Thought {
                        text: text.to_owned(),
                    });
                }
            }
            "tool_call" | "tool_call_update" => self.tool(update, step),
            "plan" => {
                let lines: Vec<String> = blocks(&update["entries"])
                    .iter()
                    .map(|entry| {
                        let mark = match entry["status"].as_str() {
                            Some("completed") => "x",
                            _ => " ",
                        };
                        format!("- [{mark}] {}", text_of(&entry["content"]))
                    })
                    .collect();
                if !lines.is_empty() {
                    self.settle(step);
                    step.events.push(TurnEvent::Text {
                        text: lines.join("\n"),
                    });
                }
            }
            "usage_update" => {
                if update["cost"]["currency"] == "USD" {
                    self.cost = update["cost"]["amount"].as_f64().or(self.cost);
                }
            }
            "config_option_update" => {
                self.options_are(&update["configOptions"], step);
                // The first one after the session says every provider's
                // models are known now.
                if !self.models_known {
                    self.models_known = true;
                    self.advance(step);
                }
            }
            "current_mode_update" => {
                let mode = text_of(&update["currentModeId"]);
                self.mode_is(&mode, step);
            }
            _ => {}
        }
    }
}

/// The answer to a permission request.
fn outcome(id: &Value, outcome: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": { "outcome": outcome } }).to_string()
}

/// The id of a request as text: the same for the number and for its digits.
fn id_text(id: &Value) -> String {
    match id {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The changes of files a tool call carries, as a chat shows them: under
/// each path, the lines that went after `- ` and the lines that came after
/// `+ `; a new file is its first lines. Empty when it carries none.
fn changes_detail(call: &Value) -> String {
    let files: Vec<String> = blocks(&call["content"])
        .iter()
        .filter(|block| block["type"] == "diff")
        .map(|block| {
            let path = text_of(&block["path"]);
            let new = text_of(&block["newText"]);
            match block["oldText"].as_str() {
                Some(old) if !old.is_empty() => edit_detail(&path, old, &new),
                _ => written_detail(&path, &new, '+'),
            }
        })
        .collect();
    cap_detail(&files.join("\n\n"))
}

/// What is asked, in full: the change of a file as old and new, or what
/// the tool was given.
fn detail_of(call: &Value) -> String {
    let changes: Vec<String> = blocks(&call["content"])
        .iter()
        .filter(|block| block["type"] == "diff")
        .map(|block| {
            let path = text_of(&block["path"]);
            let new = text_of(&block["newText"]);
            match block["oldText"].as_str() {
                Some(old) if !old.is_empty() => {
                    format!("{path}\n--- before\n{old}\n+++ after\n{new}")
                }
                _ => format!("{path}\n+++ new file\n{new}"),
            }
        })
        .collect();
    if !changes.is_empty() {
        return clip(&changes.join("\n\n"), DETAIL_CHARS);
    }
    clip(
        &serde_json::to_string_pretty(&call["rawInput"]).unwrap_or_default(),
        DETAIL_CHARS,
    )
}

impl AgentSession for OpencodeSession {
    fn first(&mut self, turn: &TurnSpec<'_>) -> Vec<Outgoing> {
        self.prompt = Some(turn.prompt.to_owned());
        let wanted = |text: Option<&str>| text.filter(|text| !text.is_empty()).map(str::to_owned);
        if let Some(value) = wanted(turn.model) {
            self.waits_for_models = true;
            self.todo.push_back(Setting {
                id: ID_MODEL,
                option: "model",
                value,
            });
        }
        if let Some(value) = wanted(turn.effort) {
            self.waits_for_models = true;
            self.todo.push_back(Setting {
                id: ID_EFFORT,
                option: "effort",
                value,
            });
        }
        if let Some(mode) = wanted(turn.mode) {
            self.auto = mode == MODE_AUTO;
            self.todo.push_back(Setting {
                id: ID_MODE,
                option: "mode",
                value: if self.auto {
                    AUTO_AGENT.to_owned()
                } else {
                    mode
                },
            });
        }
        let folder = turn.cwd.filter(|cwd| !cwd.is_empty()).unwrap_or("/");
        let session = match wanted(turn.resume) {
            Some(id) => {
                self.resumed = true;
                self.session = Some(id.clone());
                request(
                    ID_SESSION,
                    "session/resume",
                    json!({ "sessionId": id, "cwd": folder, "mcpServers": [] }),
                )
            }
            None => request(
                ID_SESSION,
                "session/new",
                json!({ "cwd": folder, "mcpServers": [] }),
            ),
        };
        vec![
            Outgoing {
                tag: TAG_HELLO.to_owned(),
                line: hello(),
            },
            Outgoing {
                tag: TAG_SESSION.to_owned(),
                line: session,
            },
        ]
    }

    fn line(&mut self, value: &Value, step: &mut Step) {
        match (value["method"].is_string(), value["id"].is_null()) {
            (true, false) => self.ask(value, step),
            (true, true) => self.told(value, step),
            (false, false) => self.answered(value, step),
            (false, true) => {}
        }
    }

    fn noise(&mut self, line: &str) {
        let text = line.trim();
        if !text.is_empty() {
            if self.noise.len() == NOISE_LINES {
                self.noise.pop_front();
            }
            self.noise.push_back(clip(text, 200));
        }
    }

    fn delivered(&mut self, tag: &str, step: &mut Step) {
        if tag == TAG_TURN {
            self.prompt_delivered = true;
        }
        if let Some(id) = tag_id('a', tag) {
            self.resolve(&id, step);
        }
    }

    fn encode(&mut self, action: Action) -> Result<Vec<Outgoing>, MobileError> {
        match action {
            // A second prompt while one runs is refused by opencode.
            Action::Prompt(_) => Err(MobileError::NotSupported),
            Action::Answer(id, answer) => Ok(vec![Outgoing {
                tag: id_tag('a', &id),
                line: self.answer(&id, answer)?,
            }]),
            Action::Interrupt => {
                if self.finished {
                    return Err(MobileError::TurnEnded);
                }
                if self.session.is_none() {
                    self.stop_wanted = true;
                    return Ok(Vec::new());
                }
                Ok(self.cancel())
            }
            Action::SetMode(mode) => {
                if self.finished {
                    return Err(MobileError::TurnEnded);
                }
                self.auto = mode == MODE_AUTO;
                let value = if self.auto { AUTO_AGENT } else { mode.as_str() };
                let id = self.next_id("mode");
                let line = self
                    .set(&id, "mode", value)
                    .ok_or(MobileError::NotSupported)?;
                Ok(vec![Outgoing {
                    tag: format!("m.{id}"),
                    line,
                }])
            }
            Action::SetModel(model) => {
                if self.finished {
                    return Err(MobileError::TurnEnded);
                }
                let id = self.next_id("model");
                let line = self
                    .set(&id, "model", &model)
                    .ok_or(MobileError::NotSupported)?;
                Ok(vec![Outgoing {
                    tag: format!("m.{id}"),
                    line,
                }])
            }
        }
    }

    fn idle(&self) -> bool {
        self.idle && self.asked.is_empty()
    }

    fn exited(&mut self, code: u32) -> Vec<TurnEvent> {
        let mut events: Vec<TurnEvent> = std::mem::take(&mut self.asked)
            .into_keys()
            .map(|id| TurnEvent::QuestionResolved { id })
            .collect();
        if !self.finished {
            self.finished = true;
            let error = if self.noise.is_empty() {
                format!("the agent stopped with exit code {code}")
            } else {
                self.noise.iter().cloned().collect::<Vec<_>>().join("\n")
            };
            events.push(TurnEvent::Finished {
                ok: false,
                error: Some(error),
                cost_usd: None,
                duration_ms: None,
                denied_tools: Vec::new(),
                interrupted: false,
            });
        }
        events.push(TurnEvent::Exited { code });
        events
    }
}

/// The command that asks opencode what it offers, without starting a
/// conversation: its version, the models it lists and its agents.
pub(crate) const OPTIONS_SCRIPT: &str = r#"program=$1
printf 'leon-version:%s\n' "$("$program" --version 2>/dev/null | tail -n 1)"
models=$("$program" models 2>/dev/null)
if [ -z "$models" ]; then
    # Asked in a folder it has not worked in yet, it may answer before it
    # knows its providers.
    sleep 1
    models=$("$program" models 2>/dev/null)
fi
printf 'leon-models:\n%s\n\nleon-agents:\n' "$models"
"$program" debug agents 2>/dev/null
exit 0"#;

/// What [`OPTIONS_SCRIPT`] is given after the program: nothing.
pub(crate) fn options_arguments() -> Vec<String> {
    Vec::new()
}

/// The efforts offered for every model. Which of them a model has is only
/// known inside a conversation; one it lacks is left as that computer has it.
const EFFORTS: &[&str] = &["low", "medium", "high", "max"];

/// Reads the output of [`OPTIONS_SCRIPT`].
pub(crate) fn parse_options(text: &str) -> AgentOptions {
    let mut options = AgentOptions {
        questions: true,
        follow_ups: false,
        live_changes: true,
        // Until it listed its agents and its models, nothing is known.
        assumed: true,
        ..AgentOptions::default()
    };
    let (mut part, mut agents) = ("", String::new());
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(version) = line.strip_prefix("leon-version:") {
            // `opencode v2.0.24`: the number alone.
            options.version = version
                .split_whitespace()
                .last()
                .map(|word| word.trim_start_matches('v').to_owned())
                .filter(|text| !text.is_empty());
            continue;
        }
        match line {
            "leon-models:" => part = "models",
            "leon-agents:" => part = "agents",
            _ if part == "agents" => {
                agents.push_str(line);
                agents.push('\n');
            }
            _ if part == "models" => {
                // `provider/model`, nothing else on the line.
                let id = line.trim();
                let Some((provider, name)) = id.split_once('/') else {
                    continue;
                };
                if provider.is_empty() || name.is_empty() || id.contains(char::is_whitespace) {
                    continue;
                }
                options.models.push(AgentModel {
                    id: id.to_owned(),
                    label: format!("{name} · {provider}"),
                    description: None,
                    efforts: EFFORTS.iter().map(|effort| (*effort).to_owned()).collect(),
                });
            }
            _ => {}
        }
    }
    let listed = agents
        .find('[')
        .and_then(|at| serde_json::from_str::<Value>(agents[at..].trim()).ok());
    for agent in listed.as_ref().map(blocks).unwrap_or_default() {
        // The agents a person works with; the others are their helpers.
        if agent["mode"] != "primary" || agent["hidden"].as_bool() == Some(true) {
            continue;
        }
        let Some(id) = agent["id"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        options.modes.push(AgentMode {
            id: id.to_owned(),
            label: agent["name"].as_str().unwrap_or(id).to_owned(),
            description: agent["description"]
                .as_str()
                .filter(|text| !text.is_empty())
                .map(str::to_owned),
            dangerous: false,
        });
    }
    if options.modes.iter().any(|mode| mode.id == AUTO_AGENT) {
        options.default_mode = Some(AUTO_AGENT.to_owned());
        options.modes.push(AgentMode {
            id: MODE_AUTO.to_owned(),
            label: "Build, never asking".to_owned(),
            description: Some(
                "Every permission it asks for is allowed at once, as `opencode --auto` does."
                    .to_owned(),
            ),
            dangerous: true,
        });
    }
    options.assumed = options.modes.is_empty() && options.models.is_empty();
    options
}

#[cfg(test)]
mod tests;
