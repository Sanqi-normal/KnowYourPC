use std::sync::Mutex;

use crate::models::NodeDto;

pub struct AppState {
    pub tree: Mutex<Option<Vec<NodeDto>>>,
    pub root_path: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tree: Mutex::new(None),
            root_path: Mutex::new(None),
        }
    }
}
