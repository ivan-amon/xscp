use std::{
    collections::HashSet,
    sync::Mutex,
};
use xscp::XscpResponse;
use super::storage::store_session;

pub fn auth(
    source: String,
    auth_attempts: u8,
    sessions: &Mutex<HashSet<String>>,
) -> XscpResponse<'static> {

    if auth_attempts >= 2 {
        return XscpResponse::try_new(402, "Too Many Attempts").unwrap();
    }

    let mut guard = sessions.lock().unwrap();

    match store_session(source, &mut guard) {
        Ok(_)  => XscpResponse::try_new(200, "Login Successful").unwrap(),
        Err(_) => XscpResponse::try_new(401, "Invalid Credentials").unwrap(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn correct_auth() {
        let sessions = Mutex::new(HashSet::<String>::new());
        let response = auth("Test".to_string(), 0, &sessions);
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.reason_phrase(), "Login Successful");
    }

    #[test]
    fn invalid_auth() {
        let sessions = Mutex::new(HashSet::<String>::new());
        sessions.lock().unwrap().insert("Test".to_string());
        let response = auth("Test".to_string(), 0, &sessions);
        assert_eq!(response.status_code(), 401);
        assert_eq!(response.reason_phrase(), "Invalid Credentials");
    }

    #[test]
    fn too_many_auth_attempts() {
        let sessions = Mutex::new(HashSet::<String>::new());
        let response = auth("Test".to_string(), 3, &sessions);
        assert_eq!(response.status_code(), 402);
        assert_eq!(response.reason_phrase(), "Too Many Attempts");
    }
}
