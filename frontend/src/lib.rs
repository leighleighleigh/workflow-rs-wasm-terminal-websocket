use wasm_bindgen::convert::LongRefFromWasmAbi;
use wasm_bindgen::prelude::*;
use workflow_rs::wasm::prelude::ObjectTrait;
use std::sync::mpsc;
// use workflow_terminal::error::Error;
//websocket imports
use std::{sync::Arc, time::Duration};
// use workflow_html::{Render,html};
use async_trait::async_trait;
use std::sync::Mutex;

// use serde::{Deserialize, Serialize};
extern crate serde_json;

use workflow_log::*;
use workflow_rs::core as workflow_core;
use workflow_rs::log as workflow_log;
use workflow_rs::terminal as workflow_terminal;
use workflow_terminal::{parse, Cli, Result as WResult, Terminal};

#[wasm_bindgen]
struct ExampleCli {
    term: Arc<Mutex<Option<Arc<Terminal>>>>,
}

#[wasm_bindgen]
impl ExampleCli {
    fn new() -> Self {
        ExampleCli {
            term: Arc::new(Mutex::new(None)),
        }
    }

    fn term(&self) -> Option<Arc<Terminal>> {
        self.term.lock().unwrap().as_ref().cloned()
    }
}

impl Sink for ExampleCli {
    fn write(&self, _target: Option<&str>, _level: Level, args: &std::fmt::Arguments<'_>) -> bool {
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

    async fn digest(self: Arc<Self>, _term: Arc<Terminal>, cmd: String) -> WResult<()> {
        // called when we have some input data to handle - triggered by Enter
        try_call_on_input_callback(cmd);
        Ok(())
    }

    async fn complete(self: Arc<Self>,_term: Arc<Terminal>,cmd: String,) -> WResult<Option<Vec<String>>> {
        let argv = parse(&cmd);

        if argv.is_empty() {
            return Ok(None);
        }

        Ok(None)
    }

    fn prompt(&self) -> Option<String> {
        None
    }
}


pub fn try_call_on_input_callback(cmd : String) {
    // see if the javascript side attached anything to the 
    // wasmtty.fn_oninput variable.
    // if so, call it as a function with a single string input.
    // otherwise, print to console.
    let tty = js_sys::Reflect::get(&JsValue::from(web_sys::window().unwrap()), &JsValue::from("wasmtty"));
    match tty {
        Ok(tty) => {
            let cbfn = js_sys::Reflect::get(&tty,&JsValue::from("fn_oninput"));
            match cbfn {
                Ok(cb) => {
                    let func : js_sys::Function = cb.into();
                    let cmds : JsValue = cmd.into();
                    let _ = func.call1(&JsValue::null(), &cmds);
                },
                Err(_) => {
                    log_info!("No oninput callback found for wasmtty");
                }
            }
        },
        Err(_) => {
            log_info!("No window.wasmtty found");
        }
    }
}




#[wasm_bindgen(start)]
pub async fn main() -> Result<(), String> {
    console_error_panic_hook::set_once();

    // setup a terminal as a log sink
    let cli = Arc::new(ExampleCli::new());
    let cli_ = Arc::clone(&cli);
    let term = Arc::new(Terminal::try_new(cli_, "")?);
    // start terminal
    term.init().await?;
    term.writeln("Initialising Pyodide REPL...");

    // set logging and shot it in the terminal
    // workflow_log::pipe(Some(cli.clone()));
    // workflow_log::set_log_level(LevelFilter::Info);

    // let _ = Arc::clone(&cli);
    // workflow_core::task::spawn(async move {
    // });

    // make a closure for line writing
    let tty_term = term.clone();

    let tty_write = Closure::<dyn FnMut(String)>::new(move |m : String| {
        tty_term.write(m);
    });

    let tty_term2 = term.clone();
    let tty_writeln = Closure::<dyn FnMut(String)>::new(move |m : String| {
        tty_term2.writeln(m);
    });

    let tty_term3 = term.clone();
    let tty_prompt= Closure::<dyn FnMut()>::new(move || {
        // tty_term3.prompt();
        tty_term3.refresh_prompt();
    });

    let tty_term4 = term.clone();
    let tty_inject= Closure::<dyn FnMut(String)>::new(move |m : String| {
        let _ = tty_term4.inject(m);
    });

    let tty_cli5 = term.clone();
    let tty_set_prompt = Closure::<dyn FnMut(String)>::new(move |m : String| {
        *tty_cli5.prompt.lock().unwrap() = m.clone();
    });

    // make a new object to store functions into
    let events = js_sys::Object::new();
    js_sys::Reflect::set(&events, &"fn_writeln".into(), tty_writeln.as_ref().unchecked_ref()).unwrap();
    js_sys::Reflect::set(&events, &"fn_write".into(), tty_write.as_ref().unchecked_ref()).unwrap();
    js_sys::Reflect::set(&events, &"fn_prompt".into(), tty_prompt.as_ref().unchecked_ref()).unwrap();
    js_sys::Reflect::set(&events, &"fn_inject".into(), tty_inject.as_ref().unchecked_ref()).unwrap();
    js_sys::Reflect::set(&events, &"fn_set_prompt".into(), tty_set_prompt .as_ref().unchecked_ref()).unwrap();
    // clear
    js_sys::Reflect::set(&JsValue::from(web_sys::window().unwrap()),&JsValue::from("wasmtty"),&JsValue::undefined()).unwrap();
    // set
    js_sys::Reflect::set(&JsValue::from(web_sys::window().unwrap()),&JsValue::from("wasmtty"),&events).unwrap();


    term.run().await?;

    Ok(())
}
