extern crate neovim_lib;
extern crate tree_sitter;
extern crate tree_sitter_python;

use failure::Error;
use std::collections::{VecDeque};
use neovim_lib::{Handler, Neovim, NeovimApi, RequestHandler, Session, Value};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use tree_sitter::{Language, Parser, Tree, Node};

struct MyHandler {
    tx: Sender<String>,
}

struct DepthFirst<'a> {
    nodes: VecDeque<Node<'a>>,
}

impl <'a> DepthFirst<'a> {

    fn new(tree: &'a Tree) -> Self {
        DepthFirst::from(tree.root_node())
    }

    fn from(start: Node<'a>) -> Self {
        let mut nodes = VecDeque::new();
        nodes.push_back(start);
        DepthFirst {nodes: nodes}
    }
}

impl <'a> Iterator for DepthFirst<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Node<'a>> {
        if let Some(current) = self.nodes.pop_front() {
            self.nodes.extend(current.children());
            Some(current)
        } else {
            None
        }
    }
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
    let num_lines = buffers[0].line_count(&mut nvim)?;
    let lines = buffers[0].get_lines(&mut nvim, 0, num_lines, false)?;
    let text = lines.join("\n");
    let tree = parser.parse(text, None).unwrap();
    for leaf in DepthFirst::new(&tree).filter(|n| n.child_count() == 0) {
        let highlight_group = match leaf.kind() {
            "def" => "Keyword",
            "identifier" => "Identifier",
            _ => "Normal",
        };
        nvim.call_function(
            "nvim_buf_add_highlight",
            vec![
                Value::from(0),
                Value::from(-1),
                Value::from(highlight_group),
                Value::from(leaf.start_position().row),
                Value::from(leaf.start_position().column),
                Value::from(leaf.end_position().column),
            ]
        ).unwrap();
        println!("parsed tree node kind: {} range: {:?}", leaf.kind(), leaf.range());
    }
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
