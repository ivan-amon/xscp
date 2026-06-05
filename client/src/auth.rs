//! This module creates an XSCP Login request and sends it to the server
use io::SocketIo;
use xscp::{XscpRequest, XscpResponse};

pub async fn auth(socket_io: &mut SocketIo, source: &str) -> u16 {
    let request = match XscpRequest::try_new(xscp::OpCode::Login, source, "") {
        Ok(req) => req,
        Err(_) => panic!("Todo: Handle error"),
    };
    let _ = socket_io.write(&request.to_string()).await;
    let raw = socket_io.read().await;

    let response = match raw {
        Ok(resp) => {
            match resp {
                Some(response) => { response },
                None => panic!("EOF"),
            }
        },
        Err(_) => panic!("Todo: Handle Error"),
    };

    let response = XscpResponse::parse(&response).unwrap();

    response.status_code()
}