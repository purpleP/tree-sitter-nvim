extern crate neovim_lib;

use std::sync::mpsc::Sender;
use failure::Error;
use neovim_lib::{Neovim, NeovimApi, Session, Handler, RequestHandler, Value};
use std::sync::mpsc;

struct MyHandler {
    tx: Sender<String>
}

impl RequestHandler for MyHandler {
    fn handle_request(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, Value> {
        Err(Value::from("not implemented"))
    }
}

impl Handler for MyHandler {
    
    fn handle_notify(&mut self, name: &str, _args: Vec<Value>) {
        self.tx.send(name.to_owned()).unwrap();
    }

}


fn start() -> Result<(), Error> {
    let socket_path = "/tmp/nvimsocket";
    let mut sess = Session::new_unix_socket(socket_path)?;
    let (tx, rx) = mpsc::channel();
    sess.start_event_loop_handler(MyHandler{tx: tx});
    let mut nvim = Neovim::new(sess);
    nvim.command("echom \"connected to rust client!\"")?;
    nvim.subscribe("text-changed").expect("Can not subscribe to TextChanged");
    nvim.subscribe("insert-enter").expect("Can not subscribe to InsertEnter");
    for event in rx {
        println!("event: {}", event);
    };
    Ok(())
}

fn main() {
    println!("Hello, world!");
    start().expect("Couldn't connect to neovim");
}