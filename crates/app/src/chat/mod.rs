//! Conversations with agents in Leon's own interface, instead of in the
//! agent's terminal program.
//!
//! Every agent is driven through Leon's unified agent protocol (`leon-turn`),
//! so a conversation is the same data whatever the agent is, and the window
//! draws one interface for all of them. This module is the part without a
//! window:
//!
//! * [`model`]: the conversation as a list of things to draw, built from the
//!   protocol's events;
//! * [`complete`]: what the field offers while a command, a file, a mode or
//!   a model is being named;
//! * [`turn`]: one turn on a pipe, the agent's process and its protocol.
//!
//! The window's side is `ui/chat.rs` (what the shell does) and
//! `ui/chat_view.rs` (what it draws). The setting `agent_interface` chooses
//! between this and the terminal; it is in beta and off by default.

pub mod complete;
pub mod model;
pub mod turn;

use leon_core::AgentId;
use leon_turn::Adapter;

/// The way `agent` is driven in a conversation, when the unified protocol
/// knows one. A custom agent has none and always runs in a terminal.
pub fn adapter_of(agent: AgentId) -> Option<&'static Adapter> {
    leon_turn::adapter(agent.as_str())
}

/// The value of the setting `agent_interface` that asks for conversations.
pub const UNIFIED: &str = "chat";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_agents_of_the_catalogue_can_be_driven_and_a_custom_one_cannot() {
        for agent in [AgentId::CLAUDE, AgentId::CODEX, AgentId::OPENCODE] {
            assert!(adapter_of(agent).is_some(), "{agent:?}");
        }
        let custom = AgentId::parse("my-agent").expect("a valid id");
        assert!(adapter_of(custom).is_none());
    }
}
