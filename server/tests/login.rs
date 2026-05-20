mod common;

use common::{TestClient, spawn_server};

#[tokio::test]
async fn login_ok_returns_200() {
    let addr = spawn_server().await;
    let mut client = TestClient::connect(addr).await;

    client.send("LOGN|Alice|\r\n").await;
    let response = client.recv().await.unwrap();

    assert_eq!(response, "200|Login Successful\r\n");
}
