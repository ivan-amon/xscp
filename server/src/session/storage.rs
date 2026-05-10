use std::collections::HashSet;
use xscp::XscpRequest;

/// Registers a host session by storing its source name.
///
/// # Why overengineer this?
///
/// Callers depend on the behavior ("register, reject duplicates"), not on
/// the storage being a [`HashSet`]. Hiding it behind a function lets the
/// backend evolve — richer structure, mutex-guarded, persisted to disk —
/// without touching any call site.
///
/// # Arguments
/// - `request`: login request whose source name is registered.
/// - `sessions`: set of active source names.
///
/// # Errors
/// Returns `Err` if the source name is already registered.
pub fn store_session(
    request: &XscpRequest,
    sessions: &mut HashSet<String>,
) -> Result<(), &'static str> {
    
    if !sessions.insert(request.source().to_string()) {
        return Err("Source name is already in use.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use xscp::OpCode;

    fn login(source: &str) -> XscpRequest<'_> {
        XscpRequest::try_new(OpCode::Login, source, "").unwrap()
    }

    #[test]
    fn correct_session() {
        let mut sessions = HashSet::<String>::new();
        let request = login("TEST");

        store_session(&request, &mut sessions).unwrap();

        assert!(sessions.contains("TEST"));
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn multiple_correct_sessions() {
        let mut sessions = HashSet::<String>::new();

        let request_1 = login("TEST 1");
        store_session(&request_1, &mut sessions).unwrap();

        let request_2 = login("TEST 2");
        store_session(&request_2, &mut sessions).unwrap();

        assert!(sessions.contains("TEST 1"));
        assert!(sessions.contains("TEST 2"));
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn source_taken() {
        let mut sessions = HashSet::<String>::new();

        let request_1 = login("TEST");
        store_session(&request_1, &mut sessions).unwrap();

        let request_2 = login("TEST");
        let err = store_session(&request_2, &mut sessions).unwrap_err();

        assert_eq!("Source name is already in use.", err);
        assert_eq!(sessions.len(), 1);
    }
}
