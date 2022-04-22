use std::{
    collections::{HashMap, VecDeque},
    io::ErrorKind,
    net::{TcpListener, TcpStream},
};

use commonr::{data::*, net};
use macroquad::{
    hash,
    prelude::*,
    ui::{root_ui, widgets::Group},
};

// TODO RA doesn't work on this file???

#[derive(Debug)]
struct Server {
    listener: TcpListener,
    connection: Option<Connection>,
    msgs: Vec<Update>,
}

impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:26000").unwrap();
        listener.set_nonblocking(true).unwrap();

        Self {
            listener,
            connection: None,
            msgs: Vec::new(),
        }
    }

    /// Accept a new connection and/or read from it.
    fn receive(&mut self) {
        if self.connection.is_none() {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    stream.set_nodelay(true).unwrap();
                    stream.set_nonblocking(true).unwrap();
                    println!("accept {}", addr);
                    self.connection = Some(Connection {
                        stream,
                        buffer: VecDeque::new(),
                    });
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                Err(err) => {
                    panic!("network error (accept): {}", err);
                }
            }
        }

        if let Some(conn) = &mut self.connection {
            let closed = net::receive(&mut conn.stream, &mut conn.buffer, &mut self.msgs);
            if closed {
                self.connection = None;
            }
        }
    }
}

#[derive(Debug)]
struct Connection {
    stream: TcpStream,
    buffer: VecDeque<u8>,
}

#[derive(Debug)]
struct State {
    globals: Globals,
    sexprecs: Vec<Sexprec>,
    nodes: HashMap<String, Node>,

    // Macroquad decided to roll its own math lib. Predictably, it's bad.
    // There's no way to const init Vec2 so i just store the "constants" here.
    // TODO remove?
    box_size: Vec2,
    box_delta: Vec2,

    offset: Vec2,
    prev_mouse_pos: Vec2,
}

impl State {
    fn new(globals: Globals, sexprecs: Vec<Sexprec>, nodes: HashMap<String, Node>) -> Self {
        let box_size = vec2(950.0, 290.0);
        let box_delta = box_size + vec2(100.0, 100.0);
        // This returns 0,0 until the mouse moves for the first time after opening the window
        // so the first drag can be glitchy and there's nothing i can do about it.
        let prev_mouse_pos = mouse_position().into();
        Self {
            globals,
            sexprecs,
            nodes,
            box_size,
            box_delta,
            offset: Vec2::ZERO,
            prev_mouse_pos,
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    pos: Vec2,
    links: Vec<Link>,
}

impl Node {
    fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            links: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Link {
    link_type: LinkType,
    dest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkType {
    Attrib,
    Payload(usize),
}

fn window_conf() -> Conf {
    Conf {
        window_title: "visualizr".to_owned(),
        // Setting width and height to the size of the screen or larger
        // creates a maximized window. Tested on Kubuntu 20.10.
        // Not using larger values (or i32::MAX) in case other platforms behave differently.
        window_width: 1920,
        window_height: 1080,
        // LATER Prevent resizing or handle it properly when using render targets.
        // Can't use fullscreen: true because of https://github.com/not-fl3/macroquad/issues/237.
        // Can't use window_resizable: false because Kubuntu's panel would cover the bottom part of the window.
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut server = Server::new();
    let mut state = None;
    loop {
        server.receive();

        for update in server.msgs.drain(..) {
            let new_state = handle_update(update);
            state = Some(new_state);
        }

        if let Some(state) = &mut state {
            let cur_mouse_pos = mouse_position().into();
            if is_mouse_button_down(MouseButton::Left) {
                // Would be nice to grab mouse here and teleport it to the other side
                // if it hits an edge, like in blender.
                // Unfortunately, set_cursor_grab changes sensitivity
                // and i don't see any way to change mouse position anyway.
                state.offset += cur_mouse_pos - state.prev_mouse_pos;
            }
            state.prev_mouse_pos = cur_mouse_pos;
        }

        clear_background(WHITE);

        if let Some(state) = &mut state {
            draw_tree(state);
        } else {
            draw_initial_box();
        }

        next_frame().await
    }
}

fn handle_update(update: Update) -> State {
    // Debug stuff
    dbg!(&update);
    let mut incoming_ptrs = HashMap::new();
    for sexprec in &update.sexprecs {
        let node_addr = update.globals.fmt_ptr(sexprec.address);
        incoming_ptrs.entry(node_addr).or_insert(0);
        let mut ptrs = sexprec.payload.pointers();
        ptrs.push(("", sexprec.attrib));
        for (_name, ptr) in ptrs {
            let dest_addr = update.globals.fmt_ptr(ptr);
            dbg!(_name, &dest_addr);
            let cnt = incoming_ptrs.entry(dest_addr).or_insert(0);
            *cnt += 1;
        }
    }
    dbg!(incoming_ptrs);

    // Collect links between nodes
    let mut nodes = HashMap::new();
    for sexprec in &update.sexprecs {
        let mut node = Node::new();

        if !update.globals.is_global(sexprec.attrib) {
            let attrib_addr = update.globals.fmt_ptr(sexprec.attrib);
            node.links.push(Link {
                link_type: LinkType::Attrib,
                dest: attrib_addr,
            });
            let ptrs = sexprec.payload.pointers();
            for i in 0..ptrs.len() {
                let (_name, ptr) = ptrs[i];
                let dest_addr = update.globals.fmt_ptr(ptr);
                node.links.push(Link {
                    link_type: LinkType::Payload(i),
                    dest: dest_addr,
                });
            }
        }

        let node_addr = update.globals.fmt_ptr(sexprec.address);
        nodes.insert(node_addr, node);
    }

    // Layout
    // We probably can't use a general DAG layout algo because
    // we want the edges to originate from specific parts of the node
    // and we don't want to reorder children.
    // Just in case, this looks interesting: https://reposhub.com/javascript/data-visualization/erikbrinkman-d3-dag.html#examples
    let root_addr = update.globals.fmt_ptr(update.sexprecs[0].address);
    walk(&mut nodes, root_addr, vec2(50.0, 50.0));

    let state = State::new(update.globals, update.sexprecs, nodes);
    state
}

fn walk(nodes: &mut HashMap<String, Node>, current: String, pos: Vec2) -> f32 {
    let current = nodes.get_mut(&current).unwrap();
    if current.pos != Vec2::ZERO{
        // Already visited
        return current.pos.x + 1050.0; // TODO consts
    }
    current.pos = pos;
    let mut child_pos = vec2(pos.x, pos.y + 390.0);
    // clone for borrowck
    for link in current.links.clone() {
        let child_max_x = walk(nodes, link.dest, child_pos);
        child_pos.x = child_max_x;
    }
    pos.x + 1050.0
}

fn draw_initial_box() {
    draw_box(
        0,
        vec2(100.0, 100.0),
        vec2(500.0, 50.0),
        "<waiting for input from visualizr>",
    );
}

fn draw_tree(state: &mut State) {
    for sexprec in &state.sexprecs {
        let addr = state.globals.fmt_ptr(sexprec.address);
        let node = &state.nodes[&addr];

        let text = SexpFormatter(&state.globals, sexprec).to_string();
        draw_box(
            sexprec.address.0,
            node.pos + state.offset,
            state.box_size,
            &text,
        );
    }

    // let offset = 20.0;
    // draw_connection(
    //     box1_pos + vec2(offset, box_size.y),
    //     box2_pos + vec2(0.0, offset),
    // );
}

fn draw_box(id: u64, box_pos: Vec2, box_size: Vec2, text: &str) {
    // Don't draw if out of bounds.
    // LATER Does this actually affect perf?
    if box_pos.x + box_size.x < 0.0
        || box_pos.y + box_size.y < 0.0
        || box_pos.x > screen_width()
        || box_pos.y > screen_height()
    {
        return;
    }

    // We wanna allow copying the data (especially stuff like pointers) but not editing
    // so we use an Editbox but reset the text every frame.
    // There seems to be no proper/native way to allow copying from a Label
    // or disable editing in an Editbox.
    Group::new(hash!() + id, box_size)
        .position(box_pos)
        .ui(&mut root_ui(), |ui| {
            let mut text = text.to_owned();
            ui.editbox(hash!() + id, box_size - vec2(5.0, 5.0), &mut text);
        });
}

fn draw_connection(src: Vec2, dest: Vec2) {
    draw_line(src.x, src.y, src.x, dest.y, 1.0, GREEN);
    draw_line(src.x, dest.y, dest.x, dest.y, 1.0, GREEN);
}
