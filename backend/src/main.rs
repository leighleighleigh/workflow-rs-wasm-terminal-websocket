#[macro_use]
extern crate log;

use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};

use tokio_tungstenite::{
    tungstenite::{
        Message,
    },
};

use serde::Serialize;
extern crate serde_json;

mod websocket_server;
use websocket_server::*;

mod ssh_echo_server;
use ssh_echo_server::ssh_echo_server;

mod target_client;

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp(None).init();
    
    // it just chills in the background
    tokio::spawn(async move {
        ssh_echo_server().await;
    });

    // websocket server handles new connections, spawning a callback future for each tcpstream that is established.
    websocket_server().await;
}

// async fn make_ssh_connection() -> Result<Client, async_ssh2_tokio::Error> {
//     // if you want to use key auth, then use following:
//     // AuthMethod::with_key_file("key_file_name", Some("passphrase"));
//     // or
//     // AuthMethod::with_key_file("key_file_name", None);
//     // or
//     // AuthMethod::with_key(key: &str, passphrase: Option<&str>)
//     // let auth_method = AuthMethod::with_password("root");
//     // AuthMethod::with_key_file("key_file_name", None);
//     // test with a randomly generated pubkey, found in this repo
//     let auth_method = AuthMethod::with_key_file("./assets/id_rsa_testing", None);

//     let client = Client::connect(
//         ("127.0.0.1", 9922),
//         "leigh",
//         auth_method,
//         ServerCheckMethod::NoCheck,
//     ).await?;

//     Ok(client)
// }

// #[derive(Clone,Debug,Serialize)]
// struct CommandResult {
//     pub stdout : String,
//     pub exit_status : u32,
// }

// async fn run_ssh_host(msg : String) -> Result<CommandResult, async_ssh2_tokio::Error> {
//     let client = make_ssh_connection().await?;
//     let result = client.execute(&msg).await?;

//     Ok(CommandResult{stdout:result.stdout,exit_status:result.exit_status})
// }

