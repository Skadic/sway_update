use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Workspace {
    pub id: usize,
    pub num: usize,
    pub name: String,
    pub output: String,
    pub focused: bool,
    pub urgent: bool,
    // This might not exist in workspace change events
    pub visible: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WorkspaceInfo {
    pub name: String,
    pub num: usize,
    pub focused: bool,
    pub urgent: bool,
    pub visible: bool,
    pub active: bool,
}

impl WorkspaceInfo {
    pub fn new(name: &str, num: usize) -> Self {
        Self {
            name: name.to_owned(),
            num,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Window {
    pub id: usize,
    pub name: Option<String>,
    pub focused: bool,
    pub urgent: bool,
    pub pid: Option<usize>,
    pub app_id: Option<String>,
}
