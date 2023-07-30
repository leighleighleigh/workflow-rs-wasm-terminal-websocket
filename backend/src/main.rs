//! Read/Write headers on server example
//! Linux:
//! ```sh
//! RUST_LOG=debug cargo run --example server-headers
//! ```

use tokio::net::{TcpListener, TcpStream};

use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};

#[macro_use]
extern crate log;

use futures_util::{SinkExt, StreamExt};

use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};

use serde::Serialize;
extern crate serde_json;

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp(None).init();

    server().await;
}

async fn server() {
    let server = TcpListener::bind("127.0.0.1:9999").await.unwrap();

    while let Ok((stream, _)) = server.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {
    let callback = |req: &Request, mut response: Response| {
        debug!("Received a new ws handshake");
        debug!("The request's path is: {}", req.uri().path());
        debug!("The request's headers are:");
        for (ref header, _value) in req.headers() {
            debug!("* {}: {:?}", header, _value);
        }

        let headers = response.headers_mut();
        headers.append("MyCustomHeader", ":)".parse().unwrap());

        Ok(response)
    };
    let mut ws_stream = accept_hdr_async(stream, callback)
        .await
        .expect("Error during the websocket handshake occurred");

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.unwrap();

        if msg.is_text() || msg.is_binary() {
            let result : CommandResult = run_ssh_host(msg.to_string()).await.unwrap();
            let result_json : String = serde_json::to_string(&result).unwrap();
            let result_msg : Message = Message::Text(result_json);
            info!("{:?}",result_msg);
            ws_stream.send(result_msg).await.unwrap();
        }
    }
}

async fn make_ssh_connection() -> Result<Client, async_ssh2_tokio::Error> {
    // if you want to use key auth, then use following:
    // AuthMethod::with_key_file("key_file_name", Some("passphrase"));
    // or
    // AuthMethod::with_key_file("key_file_name", None);
    // or
    // AuthMethod::with_key(key: &str, passphrase: Option<&str>)
    
    // let auth_method = AuthMethod::with_password("root");
    // AuthMethod::with_key_file("key_file_name", None);
    
    
    let auth_method = AuthMethod::with_key_file("/home/leigh/.ssh/testing_key", None);

    let mut client = Client::connect(
        ("127.0.0.1", 22),
        "leigh",
        auth_method,
        ServerCheckMethod::NoCheck,
    ).await?;

    Ok(client)
}

async fn ping_ssh_host() -> Result<(), async_ssh2_tokio::Error> {

    let mut client = make_ssh_connection().await?;

    let result = client.execute("echo Hello SSH").await?;
    assert_eq!(result.stdout, "Hello SSH\n");
    assert_eq!(result.exit_status, 0);

    let result = client.execute("echo Hello Again :)").await?;
    assert_eq!(result.stdout, "Hello Again :)\n");
    assert_eq!(result.exit_status, 0);

    Ok(())
}

#[derive(Clone,Debug,Serialize)]
struct CommandResult {
    pub stdout : String,
    pub exit_status : u32,
}

async fn run_ssh_host(msg : String) -> Result<CommandResult, async_ssh2_tokio::Error> {
    let mut client = make_ssh_connection().await?;
    let result = client.execute(&msg).await?;

    Ok(CommandResult{stdout:result.stdout,exit_status:result.exit_status})
}
