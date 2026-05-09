//! Persistent "seen URLs" store shared between the daemon and the GUI.
//!
//! Backed by a JSON file at `~/.cache/papery/seen.json`. Uses FIFO eviction
//! once the entry cap is reached so we never forget the most recently-shown
//! wallpapers.

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Hard cap on stored URLs. With ~5000 unique wallpapers we'd see entirely
/// fresh content for months before any wraparound on a 30-min interval.
const MAX_ENTRIES: usize = 5000;

#[derive(Default, Serialize, Deserialize)]
struct SeenFile {
    /// FIFO order — oldest first.
    urls: VecDeque<String>,
}

pub struct SeenStore {
    path: PathBuf,
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl SeenStore {
    pub fn load(cache_dir: &Path) -> Self {
        let path = cache_dir.join("seen.json");
        let (set, order) = match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<SeenFile>(&s) {
                Ok(f) => {
                    let set: HashSet<String> = f.urls.iter().cloned().collect();
                    (set, f.urls)
                }
                Err(_) => (HashSet::new(), VecDeque::new()),
            },
            Err(_) => (HashSet::new(), VecDeque::new()),
        };
        Self { path, set, order }
    }

    pub fn contains(&self, url: &str) -> bool {
        self.set.contains(url)
    }

    pub fn insert(&mut self, url: String) {
        if self.set.contains(&url) {
            return;
        }
        // FIFO eviction once we hit the cap
        while self.order.len() >= MAX_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.set.remove(&oldest);
            }
        }
        self.set.insert(url.clone());
        self.order.push_back(url);
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = SeenFile {
            urls: self.order.clone(),
        };
        if let Ok(json) = serde_json::to_string(&file) {
            // Atomic write: write to temp, rename
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }
}
