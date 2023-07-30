use wasm_bindgen::prelude::*;
// use workflow_terminal::error::Error;
//websocket imports
use std::{sync::Arc, time::Duration};
// use workflow_html::{Render,html};
use async_trait::async_trait;
use std::sync::Mutex;


#[cfg(feature = "release")]
use workflow_rs::log as workflow_log;
#[cfg(feature = "release")]
use workflow_rs::core as workflow_core;
#[cfg(feature = "release")]
use workflow_rs::websocket as workflow_websocket;
#[cfg(feature = "release")]
use workflow_rs::terminal as workflow_terminal;

use workflow_terminal::{Terminal,parse,Cli,Result as WResult};
use workflow_log::*;
use workflow_websocket::client::{Message, Options, WebSocket, ConnectOptions};

struct ExampleCli {
    term: Arc<Mutex<Option<Arc<Terminal>>>>,
    ws: Arc<Mutex<Option<Arc<WebSocket>>>>,
}

impl ExampleCli {
    fn new() -> Self {
        ExampleCli {
            term: Arc::new(Mutex::new(None)),
            ws: Arc::new(Mutex::new(None)),
        }
    }

    fn term(&self) -> Option<Arc<Terminal>> {
        self.term.lock().unwrap().as_ref().cloned()
    }

    fn ws(&self) -> Option<Arc<WebSocket>> {
        self.ws.lock().unwrap().as_ref().cloned()
    }

    fn set_ws(&self, ws: Arc<WebSocket>) {
        *self.ws.lock().unwrap() = Some(ws);
    }

    async fn disconnect(&self) {
        // if it was open, close it
        if let Some(ws) = self.ws() {
            if ws.is_open() {
                match ws.disconnect().await {
                    Ok(_) => {
                        log_info!("Disconnected from server");
                    }
                    Err(err) => {
                        log_error!("Error disconnecting from server: {}", err);
                    }
                };
            }
        }

        *self.ws.lock().unwrap() = None;
    }

    async fn connect(&self, ip: &str, port: u16) {
        // if it was open, close it
        self.disconnect().await;

        // make a websocket connection
        let ws : Option<WebSocket> = match WebSocket::new(&format!("ws://{}:{}", ip, port), Options::default(), None) {
            Ok(ws) => {
                log_info!("Websocket created");
                Some(ws)
            }
            Err(err) => {
                log_error!("Error creating websocket: {}", err);
                None
            }
        };

        let ws : WebSocket = ws.unwrap();

        match ws.connect(ConnectOptions::default()).await {
            Ok(_) => {
                log_info!("Connected to server");
            }
            Err(err) => {
                log_error!("Error connecting to server: {}", err);
            }
        };

        self.set_ws(Arc::new(ws));
    }
}

impl Sink for ExampleCli {
    fn write(&self, _target: Option<&str>, _level: Level, args: &std::fmt::Arguments<'_>) -> bool {
        // note, the terminal may not be initialized
        // if workflow_log::pipe() is bound before the
        // Terminal::init() is complete.
        if let Some(term) = self.term() {
            term.writeln(args.to_string());
            // true to disable further processing (no further output is made)
            true
        } else {
            // false for default log output handling (print to stdout or web console)
            false
        }
    }
}

#[async_trait]
impl Cli for ExampleCli {
    fn init(self: Arc<Self>, term: &Arc<Terminal>) -> WResult<()> {
        *self.term.lock().unwrap() = Some(term.clone());
        Ok(())
    }

    async fn digest(self: Arc<Self>, term: Arc<Terminal>, cmd: String) -> WResult<()> {
        let argv = parse(&cmd);
        match argv[0].as_str() {
            "help" => {
                let commands = vec![
                    "help - this list",
                    "hello - simple text output",
                    "test - log_trace!() macro output",
                    "history - list command history",
                    "sleep - sleep for 5 seconds",
                    "ask - ask user for text input (with echo)",
                    "pass - ask user for password text input (no echo)",
                    "connect <ip> <port> - connect to a websocket server",
                    "disconnect - disconnect from websocket server",
                    "send <message> - send a message to the websocket connection",
                    "exit - exit terminal",
                ];
                term.writeln("\n\rCommands:\n\r");
                term.writeln("\t".to_string() + &commands.join("\n\r\t") + "\n\r");
            }
            "hello" => {
                term.writeln("hello back to you!");
            }
            "history" => {
                let history = term.history();
                for line in history.iter() {
                    term.writeln(line);
                }
            }
            "test" => {
                log_trace!("log_trace!() macro test");
            }
            "sleep" => {
                log_trace!("start sleep (5 sec)");
                workflow_core::task::sleep(Duration::from_millis(5000)).await;
                log_trace!("finish sleep");
            }
            "ask" => {
                let text = term.ask(false, "Enter something:").await?;
                log_info!("You have entered something: {}", text);
            }
            "pass" => {
                let text = term.ask(true, "Enter something:").await?;
                log_info!("You have entered something: {}", text);
            }
            "connect" => {
                // get the ip and port to connect to
                if argv.len() < 3 {
                    log_error!("Please specify an ip and port to connect to");
                    return Ok(());
                }

                // collect all the following arguments
                let ip = argv[1].clone();
                let port = argv[2].parse::<u16>().unwrap();
                
                // connect!
                self.connect(&ip, port).await;
            }
            "disconnect" => {
                // clear the connection
                self.disconnect().await;
                return Ok(());
            }
            "send" => {
                // get our websocket connection
                let ws_ = match self.ws() {
                    Some(ws) => ws,
                    None => {
                        log_error!("No websocket connection");
                        return Ok(());
                    }
                };

                // get the message to send
                if argv.len() < 2 {
                    log_error!("Please specify a message to send");
                    return Ok(());
                }

                // collect all the following arguments
                let msg = argv[1..].join(" ");
                
                // send it!
                match  ws_.send(Message::Text(msg.clone())).await {
                    Ok(_) => {
                        log_info!("▷ {msg}");
                    }
                    Err(err) => {
                        log_error!("Error sending message: {}", err);
                    }
                }
            }
            "exit" => {
                term.writeln("bye!");
                term.exit().await;
            }
            _ => return Err(format!("command not found: {cmd}").into()),
        }

        Ok(())
    }

    async fn complete(
        self: Arc<Self>,
        _term: Arc<Terminal>,
        cmd: String,
    ) -> WResult<Option<Vec<String>>> {
        let argv = parse(&cmd);
        if argv.is_empty() {
            return Ok(None);
        }
        let last = argv.last().unwrap();
        if last.starts_with('a') {
            Ok(Some(vec![
                "alpha".to_string(),
                "aloha".to_string(),
                "albatross".to_string(),
            ]))
        } else {
            Ok(None)
        }
    }

    fn prompt(&self) -> Option<String> {
        None
    }
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), String> {
    console_error_panic_hook::set_once();

    // setup a terminal as a log sink 
    let cli = Arc::new(ExampleCli::new());

    // clone the cli sink, and move it into the task to create the terminal
    let cli_ = Arc::clone(&cli);
    let term = Arc::new(Terminal::try_new(cli_, "$ ")?);
    term.init().await?;
    term.writeln("Terminal example (type 'help' for list of commands)");

    // set logging and shot it in the terminal
    workflow_log::pipe(Some(cli.clone()));
    workflow_log::set_log_level(LevelFilter::Info);
    log_info!("Logger initialized...");

    // this loop will receive messages from the websocket, and log them
    let ws_cli_ = Arc::clone(&cli);

    workflow_core::task::spawn(async move {
        loop {
            // try to clone the websocket
            let ws_ = match ws_cli_.ws() {
                Some(ws) => ws,
                None => {
                    // wait for the websocket to be created
                    workflow_core::task::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };

            // wait for the websocket to connect
            while !ws_.is_open() {
                workflow_core::task::sleep(Duration::from_millis(100)).await;
            }

            loop {
                // check if the websocket is still connected
                if !ws_.is_open() {
                    log_error!("Websocket disconnected");
                    break;
                }
                let message = ws_.recv().await.unwrap();
                log_info!("◁ receiving message: {:?}", message);
            }
        }
    });

    term.run().await?;

    Ok(())
}
