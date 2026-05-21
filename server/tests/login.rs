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

#[tokio::test]
async fn login_invalid_credentials_returns_401() {
    let addr = spawn_server().await;

    let mut client_1 = TestClient::connect(addr).await;
    let mut client_2 = TestClient::connect(addr).await;

    client_1.send("LOGN|Alice|\r\n").await;
    client_2.send("LOGN|Alice|\r\n").await;
    let response = client_2.recv().await.unwrap();

    assert_eq!(response, "401|Invalid Credentials\r\n");
}

#[tokio::test]
async fn login_exceeded_attempts_returns_402() {
    let addr = spawn_server().await;

    let mut client_1 = TestClient::connect(addr).await;
    let mut client_2 = TestClient::connect(addr).await;
    
    client_1.send("LOGN|Alice|\r\n").await;

    client_2.send("LOGN|Alice|\r\n").await;
    client_2.send("LOGN|Alice|\r\n").await;
    client_2.send("LOGN|Alice|\r\n").await;

    client_2.recv().await;
    client_2.recv().await;
    let response = client_2.recv().await.unwrap();

    println!("{response}");
    assert_eq!(response, "402|Too Many Attempts\r\n");
}
