use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use russh::*;
use russh::client::Msg;
use russh_keys::*;
use tokio::net::ToSocketAddrs;

pub struct Client {}

#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        self,
        _server_public_key: &key::PublicKey,
    ) -> Result<(Self, bool), Self::Error> {
        Ok((self, true))
    }
}

pub struct Session {
    session: client::Handle<Client>,
}

impl Session {
    pub async fn connect<P: AsRef<Path>, A: ToSocketAddrs>(
        key_path: P,
        user: impl Into<String>,
        addrs: A,
    ) -> Result<Self> {
        let key_pair = load_secret_key(key_path, None)?;
        let config = client::Config {
            connection_timeout: Some(Duration::from_secs(5)),
            ..<_>::default()
        };
        let config = Arc::new(config);
        let sh = Client {};
        let mut session = client::connect(config, addrs, sh).await?;
        let _auth_res = session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await?;
        Ok(Self { session })
    }

    pub async fn call(&mut self, command: &str) -> Result<CommandResult> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;
        let mut output = Vec::new();
        let mut code = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    output.write_all(data).unwrap();
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                }
                _ => {}
            }
        }
        Ok(CommandResult { output, code })
    }

    pub async fn get_channel(&mut self) -> Result<Channel<Msg>> {
        let channel = self.session.channel_open_session().await?;
        Ok(channel)
    }

    pub async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub output: Vec<u8>,
    pub code: Option<u32>,
}

impl CommandResult {
    pub fn output(&self) -> String {
        String::from_utf8_lossy(&self.output).into()
    }

    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}