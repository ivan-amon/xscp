use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub type Sessions = Arc<Mutex<HashSet<String>>>;

/// Registers a host session by storing its source name.
///
/// # Arguments
/// - `source`: source name to register.
/// - `sessions`: set of active source names.
///
/// # Errors
/// Returns `Err` if the source name is already registered.
pub fn store_session(source: String, sessions: &mut HashSet<String>) -> Result<(), &'static str> {
    if !sessions.insert(source) {
        return Err("Source name is already in use");
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn correct_session() {
        let mut sessions = HashSet::<String>::new();
        assert!(!sessions.contains("Test"));
        store_session("Test".to_string(), &mut sessions).unwrap();
        assert!(sessions.contains("Test"));
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn multiple_correct_sessions() {
        let mut sessions = HashSet::<String>::new();

        store_session("Test 1".to_string(), &mut sessions).unwrap();
        store_session("Test 2".to_string(), &mut sessions).unwrap();

        assert!(sessions.contains("Test 1"));
        assert!(sessions.contains("Test 2"));
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn source_taken() {
        let mut sessions = HashSet::<String>::new();
        store_session("Test".to_string(), &mut sessions).unwrap();
        let err = store_session("Test".to_string(), &mut sessions).unwrap_err();

        assert_eq!("Source name is already in use", err);
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains("Test"));
    }
}
