
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ValueEnum
)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        };

        write!(f, "{}", value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub content: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: Priority,
    pub done: bool,
}

impl Task {
    pub fn new(
        content: String,
        description: Option<String>,
        deadline: Option<String>,
        priority: Option<Priority>,
    ) -> Self {
        Self {
            content,
            description,
            deadline,
            priority: priority.unwrap_or_default(),
            done: false,
        }
    }
}
