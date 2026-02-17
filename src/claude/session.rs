//! Session manager for multi-step cycles
//!
//! Maps session tags (human-readable names in cycles.toml) to Claude Code
//! session IDs (obtained from the `SystemInit` stream event). Steps sharing
//! the same session tag continue the same Claude Code conversation.

use std::collections::HashMap;

/// Manages session tag → session ID mapping for one cycle execution.
///
/// Session tags are scoped to a single cycle execution — a new `SessionManager`
/// is created for each cycle run, so sessions never persist across iterations.
#[derive(Debug, Default)]
pub struct SessionManager {
    /// Maps session tag → Claude Code session ID
    tag_to_id: HashMap<String, String>,
}

impl SessionManager {
    /// Create a new empty session manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a session ID for a given tag.
    ///
    /// Called after a step's `SystemInit` event is received, providing the
    /// real Claude Code session ID that should be used for resumption.
    pub fn register(&mut self, tag: &str, session_id: String) {
        self.tag_to_id.insert(tag.to_string(), session_id);
    }

    /// Look up the session ID for a tag, if any.
    #[must_use]
    pub fn get_session_id(&self, tag: &str) -> Option<&str> {
        self.tag_to_id.get(tag).map(String::as_str)
    }

    /// Build extra CLI args for Claude Code to resume an existing session.
    ///
    /// Returns `["--resume", "<session_id>"]` if the tag has a previously
    /// registered session, or an empty `Vec` for a fresh session.
    #[must_use]
    pub fn resume_args(&self, session_tag: Option<&str>) -> Vec<String> {
        let Some(tag) = session_tag else {
            return vec![];
        };
        let Some(session_id) = self.get_session_id(tag) else {
            return vec![];
        };
        vec!["--resume".to_string(), session_id.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session_manager_is_empty() {
        let mgr = SessionManager::new();
        assert!(mgr.get_session_id("architect").is_none());
    }

    #[test]
    fn test_register_and_lookup_session() {
        let mut mgr = SessionManager::new();
        mgr.register("architect", "abc-123".to_string());
        assert_eq!(mgr.get_session_id("architect"), Some("abc-123"));
    }

    #[test]
    fn test_lookup_unknown_tag_returns_none() {
        let mgr = SessionManager::new();
        assert!(mgr.get_session_id("nonexistent").is_none());
    }

    #[test]
    fn test_register_overwrites_existing_session() {
        let mut mgr = SessionManager::new();
        mgr.register("architect", "old-id".to_string());
        mgr.register("architect", "new-id".to_string());
        assert_eq!(mgr.get_session_id("architect"), Some("new-id"));
    }

    #[test]
    fn test_multiple_tags_are_independent() {
        let mut mgr = SessionManager::new();
        mgr.register("architect", "id-arch".to_string());
        mgr.register("coder", "id-coder".to_string());
        assert_eq!(mgr.get_session_id("architect"), Some("id-arch"));
        assert_eq!(mgr.get_session_id("coder"), Some("id-coder"));
    }

    #[test]
    fn test_resume_args_no_tag() {
        let mgr = SessionManager::new();
        assert!(mgr.resume_args(None).is_empty());
    }

    #[test]
    fn test_resume_args_unknown_tag() {
        let mgr = SessionManager::new();
        // Tag not yet registered → fresh session, no --resume args
        assert!(mgr.resume_args(Some("architect")).is_empty());
    }

    #[test]
    fn test_resume_args_known_tag() {
        let mut mgr = SessionManager::new();
        mgr.register("architect", "abc-123".to_string());
        let args = mgr.resume_args(Some("architect"));
        assert_eq!(args, vec!["--resume", "abc-123"]);
    }

    #[test]
    fn test_resume_args_correct_flag_name() {
        let mut mgr = SessionManager::new();
        mgr.register("coder", "xyz-789".to_string());
        let args = mgr.resume_args(Some("coder"));
        assert_eq!(args[0], "--resume");
        assert_eq!(args[1], "xyz-789");
    }
}
