use xscp::{XscpRequest, XscpResponse};

pub fn dispatch(req: XscpRequest<'_>) -> XscpResponse<'static> {
    match req.opcode() {
        xscp::OpCode::Send => todo!(),
        xscp::OpCode::Exit => todo!(),
        _ => XscpResponse::try_new(400, "BAD REQUEST").unwrap()
    }
}