use serde::{Deserialize, Serialize};

/// AI 伙伴身份预留。当前仅 `deepseek` 有可用美术；其他身份经 `sprite`/`frame` 请求时返回 `bad_input`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Companion {
    Deepseek,
    Gpt,
    Claude,
    Grok,
    Glm,
    Kimi,
    Gemini,
}

impl Companion {
    pub const ALL: [Self; 7] = [
        Self::Deepseek,
        Self::Gpt,
        Self::Claude,
        Self::Grok,
        Self::Glm,
        Self::Kimi,
        Self::Gemini,
    ];

    pub const fn is_available(self) -> bool {
        matches!(self, Self::Deepseek)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Deepseek => "deepseek",
            Self::Gpt => "gpt",
            Self::Claude => "claude",
            Self::Grok => "grok",
            Self::Glm => "glm",
            Self::Kimi => "kimi",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CompanionAction {
    Idle,
    Blink,
    Move,
    Observe,
    Eat,
    Play,
    Sleep,
    Startled,
    Sick,
    Evolve,
    End,
    Think,
    Work,
    Parallel,
    Build,
    Error,
    Complete,
    AwaitConfirmation,
    Organize,
    Carry,
    Wake,
    PokeLeft,
    PokeRight,
    Tickle,
    Lift,
    Praise,
}

impl CompanionAction {
    pub const ALL: [Self; 26] = [
        Self::Idle,
        Self::Blink,
        Self::Move,
        Self::Observe,
        Self::Eat,
        Self::Play,
        Self::Sleep,
        Self::Startled,
        Self::Sick,
        Self::Evolve,
        Self::End,
        Self::Think,
        Self::Work,
        Self::Parallel,
        Self::Build,
        Self::Error,
        Self::Complete,
        Self::AwaitConfirmation,
        Self::Organize,
        Self::Carry,
        Self::Wake,
        Self::PokeLeft,
        Self::PokeRight,
        Self::Tickle,
        Self::Lift,
        Self::Praise,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Blink => "blink",
            Self::Move => "move",
            Self::Observe => "observe",
            Self::Eat => "eat",
            Self::Play => "play",
            Self::Sleep => "sleep",
            Self::Startled => "startled",
            Self::Sick => "sick",
            Self::Evolve => "evolve",
            Self::End => "end",
            Self::Think => "think",
            Self::Work => "work",
            Self::Parallel => "parallel",
            Self::Build => "build",
            Self::Error => "error",
            Self::Complete => "complete",
            Self::AwaitConfirmation => "await_confirmation",
            Self::Organize => "organize",
            Self::Carry => "carry",
            Self::Wake => "wake",
            Self::PokeLeft => "poke_left",
            Self::PokeRight => "poke_right",
            Self::Tickle => "tickle",
            Self::Lift => "lift",
            Self::Praise => "praise",
        }
    }

    pub(crate) const fn from_pose(pose: crate::Pose) -> Self {
        match pose {
            crate::Pose::Idle => Self::Idle,
            crate::Pose::Observe => Self::Observe,
            crate::Pose::Eat => Self::Eat,
            crate::Pose::Play => Self::Play,
            crate::Pose::Sleep => Self::Sleep,
            crate::Pose::Startled => Self::Startled,
            crate::Pose::Sick => Self::Sick,
            crate::Pose::Evolve => Self::Evolve,
            crate::Pose::End => Self::End,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    Curious,
    Focused,
    Shy,
    Happy,
    Startled,
    Smug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompanionAppearance {
    pub character: Companion,
    #[serde(default)]
    pub action: Option<CompanionAction>,
    #[serde(default)]
    pub expression: Option<Expression>,
}
