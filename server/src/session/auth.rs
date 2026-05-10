use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use xscp::{XscpRequest, XscpResponse};
use super::storage::store_session;

pub type Sessions = Arc<Mutex<HashSet<String>>>;

pub fn auth(
    request: &XscpRequest,
    auth_attempts: u8,
    sessions: &Mutex<HashSet<String>>,
) -> XscpResponse<'static> {

    if auth_attempts >= 2 {
        return XscpResponse::try_new(402, "TOO MANY ATTEMPTS").unwrap();
    }

    let mut guard = sessions.lock().unwrap();

    match store_session(&request, &mut guard) {
        Ok(_)  => XscpResponse::try_new(200, "LOGIN SUCCESSFUL").unwrap(),
        Err(_) => XscpResponse::try_new(401, "INVALID CREDENTIALS").unwrap(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn correct_auth() {
        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let sessions = Mutex::new(HashSet::<String>::new());
        let response = auth(&request, 0, &sessions);

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.reason_phrase(), "LOGIN SUCCESSFUL");
    }

    #[test]
    fn invalid_auth() {
        let sessions = Mutex::new(HashSet::<String>::new());
        sessions.lock().unwrap().insert("test".to_string());

        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let response = auth(&request, 0, &sessions);

        assert_eq!(response.status_code(), 401);
        assert_eq!(response.reason_phrase(), "INVALID CREDENTIALS");
    }

    #[test]
    fn too_many_auth_attempts() {
        let request = XscpRequest::try_new(xscp::OpCode::Login, "test", "").unwrap();
        let sessions = Mutex::new(HashSet::<String>::new());
        let response = auth(&request, 3, &sessions);

        assert_eq!(response.status_code(), 402);
        assert_eq!(response.reason_phrase(), "TOO MANY ATTEMPTS");
    }
}
