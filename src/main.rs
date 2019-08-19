extern crate neovim_lib;
extern crate tree_sitter;
extern crate tree_sitter_python;

use failure::Error;
use neovim_lib::{Handler, Neovim, NeovimApi, RequestHandler, Session, Value};
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::iter;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use tree_sitter::{Language, Node, Parser, Point, Tree};

struct MyHandler {
    tx: Sender<String>,
}

struct DepthFirst<'a> {
    nodes: VecDeque<Node<'a>>,
}

struct ColumnRange {
    line: u64,
    start_col: u64,
    end_col: Option<u64>,
}

impl<'a> DepthFirst<'a> {
    fn new(tree: &'a Tree) -> Self {
        DepthFirst::from(tree.root_node())
    }

    fn from(start: Node<'a>) -> Self {
        let mut nodes = VecDeque::new();
        nodes.push_back(start);
        DepthFirst { nodes: nodes }
    }
}

impl<'a> Iterator for DepthFirst<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Node<'a>> {
        let current = self.nodes.pop_front();
        self.nodes.extend(current.iter().flat_map(|c| c.children()));
        current
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

fn ranges(start: Point, end: Point) -> impl Iterator<Item = ColumnRange> {
    let start_row = u64::try_from(start.row).unwrap();
    let end_row = u64::try_from(end.row).unwrap();
    let start_col = u64::try_from(start.column).unwrap();
    let end_col = u64::try_from(end.column).unwrap();
    let (fst_end, lst) = if start_row == end_row {
        (Some(end_col), None)
    } else {
        (
            None,
            Some(ColumnRange {
                line: end_row,
                start_col: 0,
                end_col: Some(end_col),
            })
        )
    };
    let head = iter::once(ColumnRange {
        line: start_row,
        start_col,
        end_col: fst_end,
    });
    let tail = (start_row..end_row)
        .map(|line| ColumnRange {
            line,
            start_col: 0,
            end_col: None,
        })
        .chain(lst);
    head.chain(tail)
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
        for range in ranges(leaf.start_position(), leaf.end_position()) {
            nvim.call_function(
                "nvim_buf_add_highlight",
                vec![
                    Value::from(0),
                    Value::from(-1),
                    Value::from(highlight_group),
                    Value::from(range.line),
                    Value::from(range.start_col),
                    range.end_col.map_or(Value::from(-1), Value::from),
                ],
            )
            .unwrap();
        }
        println!(
            "parsed tree node kind: {} range: {:?}",
            leaf.kind(),
            leaf.range()
        );
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
