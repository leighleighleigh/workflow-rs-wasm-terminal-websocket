#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};

use async_ssh2_tokio::client::{Client, AuthMethod, ServerCheckMethod};

use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};


use futures_util::{SinkExt, StreamExt};

use serde::Serialize;
extern crate serde_json;

use async_trait::async_trait;
use russh::server::{Msg,Session};
use russh::*;
use russh_keys::*;



#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp(None).init();
    
    // websocket server sits in the background
    tokio::spawn(async move {
        websocket_server().await;
    });

    // ssh server is main thread
    ssh_server().await;
}

async fn ssh_server() {
    let config = russh::server::Config {
        connection_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![russh_keys::key::KeyPair::generate_ed25519().unwrap()],
        ..Default::default()
    };

    let config = Arc::new(config);

    let sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    russh::server::run(config, ("127.0.0.1", 9922), sh).await.unwrap();
}

async fn websocket_server() {
    let server = TcpListener::bind("127.0.0.1:9999").await.unwrap();

    while let Ok((stream, _)) = server.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {

    // this callback is made when a new client connects
    let callback = |req: &Request, response: Response| {
        info!("Received a new ws handshake");

        info!("The request's path is: {}", req.uri().path());

        info!("The request's headers are:");

        for (ref header, _value) in req.headers() {
            debug!("* {}: {:?}", header, _value);
        }

        //let headers = response.headers_mut();
        //headers.append("MyCustomHeader", ":)".parse().unwrap());

        Ok(response)
    };
    
    // hook the accept-header callback
    let mut ws_stream = accept_hdr_async(stream, callback).await.expect("Error during the websocket handshake occurred");
    
    // main loop, reads incoming websocket messages
    while let Some(msg) = ws_stream.next().await {
        let msg = msg.unwrap();

        if msg.is_text() || msg.is_binary() {
            // send the command to the SSH connection, obtaining the CommandResult
            let result : CommandResult = run_ssh_host(msg.to_string()).await.unwrap();
            // serialize to JSON
            let result_json : String = serde_json::to_string(&result).unwrap();
            // wrap in a websocket message
            let result_msg : Message = Message::Text(result_json);
            // print the message we are sending back
            info!("{:?}",result_msg);
            // back it goes
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
    
    
    // test with a randomly generated pubkey, found in this repo
    let auth_method = AuthMethod::with_key_file("./assets/id_rsa_testing", None);

    let client = Client::connect(
        ("127.0.0.1", 9922),
        "leigh",
        auth_method,
        ServerCheckMethod::NoCheck,
    ).await?;

    Ok(client)
}

#[derive(Clone,Debug,Serialize)]
struct CommandResult {
    pub stdout : String,
    pub exit_status : u32,
}

async fn run_ssh_host(msg : String) -> Result<CommandResult, async_ssh2_tokio::Error> {
    let client = make_ssh_connection().await?;
    let result = client.execute(&msg).await?;

    Ok(CommandResult{stdout:result.stdout,exit_status:result.exit_status})
}


#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<(usize, ChannelId), russh::server::Handle>>>,
    id: usize,
}

impl Server {
    async fn post(&mut self, data: CryptoVec) {
        let mut clients = self.clients.lock().await;
        for ((id, channel), ref mut s) in clients.iter_mut() {
            if *id != self.id {
                let _ = s.data(*channel, data.clone()).await;
            }
        }
    }
}

impl server::Server for Server {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl server::Handler for Server {
    type Error = anyhow::Error;

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            clients.insert((self.id, channel.id()), session.handle());
        }
        Ok((self, true, session))
    }

    async fn auth_publickey(
        self,
        _: &str,
        _: &key::PublicKey,
    ) -> Result<(Self, server::Auth), Self::Error> {
        Ok((self, server::Auth::Accept))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let data = CryptoVec::from(format!("Got data: {}\r\n", String::from_utf8_lossy(data)));
        self.post(data.clone()).await;
        session.data(channel, data);
        Ok((self, session))
    }
    
    /*
    async fn tcpip_forward(
        self,
        address: &str,
        port: &mut u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        let handle = session.handle();
        let address = address.to_string();
        let port = *port;
        tokio::spawn(async move {
            let mut channel = handle
                .channel_open_forwarded_tcpip(address, port, "1.2.3.4", 1234)
                .await
                .unwrap();
            let _ = channel.data(&b"Hello from a forwarded port"[..]).await;
            let _ = channel.eof().await;
        });
        Ok((self, true, session))
    }
    */
}
