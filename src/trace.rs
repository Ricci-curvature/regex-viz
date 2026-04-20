use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceKind {
    Build,
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub kind: TraceKind,
    pub input: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub description: String,
    pub nfa: Nfa,
    pub active: Vec<usize>,
    pub input_pos: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nfa {
    pub states: Vec<usize>,
    pub transitions: Vec<Transition>,
    pub start: usize,
    pub accept: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: usize,
    pub to: usize,
    pub label: String,
}
