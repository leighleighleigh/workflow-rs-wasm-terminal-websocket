use std::{future::IntoFuture, ops::DerefMut};

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{SinkExt, StreamExt, Future, TryStreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use core::future::poll_fn;
use std::task::{Context,Poll};

use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Message,
    },
};

use crate::target_client::{Session as SSHSession,CommandResult as SSHCommandResult,Client as SSHClient};

// ooh getting fancy with the rust closure passing very nice
pub async fn websocket_server() {
    let server = TcpListener::bind("127.0.0.1:9999").await.unwrap();

    while let Ok((stream, _)) = server.accept().await {
        tokio::spawn(handle_websocket_connection(stream));
    }
}


async fn handle_websocket_connection(stream: TcpStream) {

    // this callback is made when a new client connects
    let callback = |req: &Request, mut response: Response| {
        info!("Received a new ws handshake");

        info!("The request's path is: {}", req.uri().path());

        info!("The request's headers are:");

        for (ref header, _value) in req.headers() {
            debug!("* {}: {:?}", header, _value);
        }

        let headers = response.headers_mut();
        headers.append("MyCustomHeader", ":)".parse().unwrap());

        Ok(response)
    };
    
    // hook the accept-header callback, wait for it to happen
    let mut ws_stream = accept_hdr_async(stream, callback).await.expect("Error during the websocket handshake occurred");

    // start the SSH session, which goes to the echo server for now
    let host = "127.0.0.1:22";
    let key = "./assets/id_rsa_testing";

    info!("Connecting to {host}");
    info!("Key path: {key}");

    let mut ssh_sesh = SSHSession::connect(key, "leigh", host).await.unwrap();

    // let r = ssh.call("whoami").await?;
    // assert!(r.success());
    // println!("Result: {}", r.output());
    // ssh.close().await?;

    // get the channel streams
    let mut ssh_channel = ssh_sesh.get_channel().await.unwrap();

    // spawn a tokio task
    tokio::task::spawn(async move {

        loop {
            tokio::select! {
                val = ws_stream.next() => {
                    match val.unwrap() {
                        Ok(msg) => {
                            info!("WS-RX: {:?}", msg);
                            // send the data to SSH!
                            if msg.is_text() || msg.is_binary() {
                                let mbytes = msg.to_string();
                                // ssh_channel.write_all(mbytes.as_slice()).await.unwrap();
                                // ssh_channel.exec(false, mbytes).await.unwrap();
                                match ssh_channel.exec(false, mbytes.clone()).await {
                                    Ok(_) => {
                                        info!("SSH-TX: {:?}", mbytes);
                                    }
                                    Err(err) => {
                                        error!("SSH-TX: {:?}", err);
                                    }
                                }
                            }
                        }
                        Err(err) => 
                        {
                            // raise an error
                            error!("WS-RX: {:?}", err);
                            // end loop
                            break;
                        }
                    }
                }
                msg = async { 
                    let m = ssh_channel.wait().await;
                    // if it was None, then we need to wait a bit
                    if ! m.is_some() {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                    m
                } => {
                    match msg {
                        Some(msg) => {
                            info!("SSH-RX {:?}", msg);

                            match msg {
                                russh::ChannelMsg::Data { ref data } => {
                                    let mut buf = Vec::new();
                                    buf.write_all(data).await.unwrap();                                    
                                    // send out websocket
                                    ws_stream.send(Message::Binary(buf)).await.unwrap();
                                }
                                _ => {
                                    // debug it
                                    debug!("SSH-RX {:?}", msg);
                                }
                            }
                        }
                        None => {
                        }
                    }
                }
            }
        }
    }).into_future().await.unwrap();
}