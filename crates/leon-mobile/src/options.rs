//! What an agent offers, as it crosses to the application.
//!
//! Declared again from `leon-turn`, field for field (see `events.rs`).

/// A permission mode of the agent.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentMode {
    /// What [`crate::TurnRequest`] and [`crate::Turn::set_mode`] take.
    pub id: String,
    /// Words for it.
    pub label: String,
    /// What it means, in a sentence.
    pub description: Option<String>,
    /// It lets the agent do anything without asking: confirm before using.
    pub dangerous: bool,
}

/// A model the agent can answer with.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentModel {
    /// What [`crate::TurnRequest`] and [`crate::Turn::set_model`] take.
    pub id: String,
    /// Its name.
    pub label: String,
    /// What it is good for, when the agent says.
    pub description: Option<String>,
    /// The efforts it can be asked for, the lightest first; none when it has
    /// no such setting.
    pub efforts: Vec<String>,
}

/// What an agent offers on the other computer, as installed there.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct AgentOptions {
    /// Its version, when it says.
    pub version: Option<String>,
    /// Its permission modes. A turn started with none uses the one that
    /// computer is configured for.
    pub modes: Vec<AgentMode>,
    /// The mode it starts in when none is asked for, when known.
    pub default_mode: Option<String>,
    /// Its models. A turn started with none uses that computer's default.
    pub models: Vec<AgentModel>,
    /// It asks before acting and waits for [`crate::Turn::answer`].
    pub questions: bool,
    /// It can be told more while it works ([`crate::Turn::send`]) and
    /// interrupted ([`crate::Turn::interrupt`]).
    pub follow_ups: bool,
    /// Its mode and model can be changed while a turn runs.
    pub live_changes: bool,
    /// The lists are this library's own, because the agent on that computer
    /// could not be asked.
    pub assumed: bool,
}

impl From<leon_turn::AgentMode> for AgentMode {
    fn from(mode: leon_turn::AgentMode) -> Self {
        Self {
            id: mode.id,
            label: mode.label,
            description: mode.description,
            dangerous: mode.dangerous,
        }
    }
}

impl From<leon_turn::AgentModel> for AgentModel {
    fn from(model: leon_turn::AgentModel) -> Self {
        Self {
            id: model.id,
            label: model.label,
            description: model.description,
            efforts: model.efforts,
        }
    }
}

impl From<leon_turn::AgentOptions> for AgentOptions {
    fn from(options: leon_turn::AgentOptions) -> Self {
        Self {
            version: options.version,
            modes: options.modes.into_iter().map(Into::into).collect(),
            default_mode: options.default_mode,
            models: options.models.into_iter().map(Into::into).collect(),
            questions: options.questions,
            follow_ups: options.follow_ups,
            live_changes: options.live_changes,
            assumed: options.assumed,
        }
    }
}
