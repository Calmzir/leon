//! Leon's unified agent protocol: one conversation with a coding agent, the
//! same whatever the agent is.
//!
//! Claude Code, Codex and opencode each have a way of being driven by a
//! program instead of a person at a terminal: Claude Code's control protocol,
//! Codex's application server, opencode over the Agent Client Protocol. They
//! differ in everything (framing, handshake, how a permission is asked, what
//! a tool's use looks like). This crate is the one place that knows them, and
//! what comes out of it is the same for all three:
//!
//! * a **turn** is one prompt sent to an agent and everything it does about
//!   it ([`TurnSpec`]);
//! * what happens is a short list of [`TurnEvent`]s: the conversation
//!   started, words, thoughts, a tool's use and its result, a question, the
//!   end;
//! * what the agent asks is a [`QuestionKind`] (a permission, a choice, a
//!   plan) and is answered with an [`Answer`];
//! * what a person can do meanwhile is an [`Action`]: say more, answer,
//!   interrupt, change the mode or the model;
//! * what an agent offers (modes, models, efforts, whether it asks) is its
//!   [`AgentOptions`].
//!
//! A client shows these and nothing else, so every agent looks and behaves
//! alike in it. Adding an agent is one [`Adapter`] in [`agents`].
//!
//! The crate is pure: lines in, events and lines out. It starts no process
//! and knows no transport. The desktop application runs the agent on a pipe
//! (`crates/app/src/chat`); the phone's library runs it in a terminal of the
//! paired computer behind its bridge (`leon-mobile`).
//!
//! * [`agents`]: the catalogue, and the [`AgentSession`] an adapter makes;
//! * [`claude`], [`codex`], [`opencode`]: each agent's own protocol;
//! * [`events`]: the events, the questions and the answers, and [`Lines`],
//!   which cuts a stream of bytes into clean lines;
//! * [`shell`]: the login shell an agent's command is started through;
//! * [`tag`]: the names messages are acknowledged by.

pub mod agents;
pub mod claude;
pub mod codex;
pub mod error;
pub mod events;
pub mod opencode;
pub mod shell;
pub mod tag;

pub use agents::{
    adapter, agents, Action, Adapter, AgentInfo, AgentSession, Outgoing, Probe, Step, TurnSpec,
    ADAPTERS,
};
pub use claude::{AgentCommand, AgentMode, AgentModel, AgentOptions};
pub use error::TurnError;
pub use events::{
    object_on, Answer, ChoiceAnswer, ChoiceOption, ChoiceQuestion, Lines, PermissionChoice,
    PermissionEffect, QuestionKind, TurnEvent,
};
