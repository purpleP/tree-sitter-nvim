extern crate neovim_lib;
extern crate tree_sitter;
extern crate tree_sitter_python;

use failure::Error;
use neovim_lib::{Handler, Neovim, NeovimApi, RequestHandler, Session, Value};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use tree_sitter::{Language, Parser};

struct MyHandler {
    tx: Sender<String>,
}

impl RequestHandler for MyHandler {
    fn handle_request(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, Value> {
        Err(Value::from("not implemented"))
    }
}

impl Handler for MyHandler {
    fn handle_notify(&mut self, name: &str, _args: Vec<Value>) {
        println!("notified {}", name);
        self.tx.send(name.to_owned()).unwrap();
    }
}

fn start() -> Result<(), Error> {
    extern "C" {
        fn tree_sitter_python() -> Language;
    }
    let mut parser = Parser::new();
    let python = unsafe { tree_sitter_python() };
    parser.set_language(python).unwrap();
    let socket_path = "/tmp/nvimsocket";
    let mut sess = Session::new_unix_socket(socket_path)?;
    let (tx, rx) = mpsc::channel();
    sess.start_event_loop_handler(MyHandler { tx: tx });
    let mut nvim = Neovim::new(sess);
    nvim.command("echom \"connected to rust client!\"")?;
    let buffers = nvim.list_bufs().unwrap();
    let lines = buffers[0].get_lines(&mut nvim, 0, 1, true)?;
    let text = lines.join("\n");
    let tree = parser.parse(text, None).unwrap();
    println!("parsed tree {:?}", tree);
    nvim.subscribe("text-changed")
        .expect("Can not subscribe to TextChanged");
    nvim.subscribe("cursor-moved")
        .expect("Can not subscribe to CursorMoved");
    nvim.subscribe("insert-enter")
        .expect("Can not subscribe to InsertEnter");
    for event in rx {
        println!("event: {}", event);
    }
    nvim.command("echom \"rust client disconnected from neovim\"")?;
    Ok(())
}

fn main() {
    println!("Hello, world!");
    start().expect("Couldn't connect to neovim");
}
