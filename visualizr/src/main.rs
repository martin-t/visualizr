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
    offset: Vec2,
    prev_mouse_pos: Vec2,
}

impl State {
    fn new(globals: Globals, sexprecs: Vec<Sexprec>, nodes: HashMap<String, Node>) -> Self {
        // This returns 0,0 until the mouse moves for the first time after opening the window
        // so the first drag can be glitchy and there's nothing i can do about it.
        let prev_mouse_pos = mouse_position().into();
        Self {
            globals,
            sexprecs,
            nodes,
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
    dest_addr: String,
    dest_global: bool,
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
            let cnt = incoming_ptrs.entry(dest_addr).or_insert(0);
            *cnt += 1;
        }
    }
    dbg!(incoming_ptrs);

    // Collect links between nodes
    let mut nodes = HashMap::new();
    for sexprec in &update.sexprecs {
        let mut node = Node::new();

        let attrib_global = update.globals.is_global(sexprec.attrib);
        let attrib_addr = update.globals.fmt_ptr(sexprec.attrib);
        node.links.push(Link {
            link_type: LinkType::Attrib,
            dest_addr: attrib_addr,
            dest_global: attrib_global,
        });
        let ptrs = sexprec.payload.pointers();
        for i in 0..ptrs.len() {
            let (_name, ptr) = ptrs[i];
            let dest_global = update.globals.is_global(ptr);
            let dest_addr = update.globals.fmt_ptr(ptr);
            node.links.push(Link {
                link_type: LinkType::Payload(i),
                dest_addr,
                dest_global,
            });
        }

        let addr = update.globals.fmt_ptr(sexprec.address);
        nodes.insert(addr, node);
    }

    // Layout
    // We probably can't use a general DAG layout algo because
    // we want the edges to originate from specific parts of the node
    // and we don't want to reorder children.
    // Just in case, this looks interesting: https://reposhub.com/javascript/data-visualization/erikbrinkman-d3-dag.html#examples
    let root_addr = update.globals.fmt_ptr(update.sexprecs[0].address);
    let margin = walk(&mut nodes, root_addr, vec2(BOX_INIT_X, BOX_INIT_Y));

    // Put globals on the left.
    // Originally i wanted to put them at the bottom but they were hard to find if the tree was large.
    let mut global_pos = vec2(
        BOX_INIT_X - BOX_WIDTH - 2.0 * BOX_GAP,
        BOX_INIT_Y + BOX_HEIGHT + BOX_GAP,
    );
    for sexprec in &update.sexprecs {
        if update.globals.is_global(sexprec.address) {
            let addr = update.globals.fmt_ptr(sexprec.address);
            nodes.get_mut(&addr).unwrap().pos = global_pos;
            global_pos.y += BOX_HEIGHT + BOX_GAP;
        }
    }

    let state = State::new(update.globals, update.sexprecs, nodes);
    state
}

// Macroquad decided to roll its own math lib. Predictably, it's bad.
// There's no way to const init a Vec2 because none of its ctors are marked const.
const BOX_INIT_X: f32 = 500.0;
const BOX_INIT_Y: f32 = 50.0;
const BOX_WIDTH: f32 = 950.0;
const BOX_HEIGHT: f32 = 290.0;
const BOX_GAP: f32 = 100.0;

/// Returns the size of the subtree plus a gap - the bottom right cooordinate of the "margin".
fn walk(nodes: &mut HashMap<String, Node>, current_addr: String, pos: Vec2) -> Vec2 {
    let current = nodes.get_mut(&current_addr).unwrap();

    if current.pos != Vec2::ZERO {
        // Already visited
        return Vec2::ZERO;
    }
    if current_addr.ends_with(')') {
        // Global
        // TODO this is really ugly
        return Vec2::ZERO;
    }
    current.pos = pos;
    println!("walking {} pos {}", current_addr, pos);
    let mut child_pos = vec2(pos.x, pos.y + BOX_HEIGHT + BOX_GAP);
    let mut margin = vec2(pos.x + BOX_WIDTH + BOX_GAP, pos.y + BOX_HEIGHT + BOX_GAP);
    // clone for borrowck
    for link in current.links.clone() {
        let child_margin = walk(nodes, link.dest_addr, child_pos);
        child_pos.x = child_pos.x.max(child_margin.x); // Max because child might return [0, 0]
        margin = margin.max(child_margin);
    }
    // LATER might be cleaner to return width, height instead of global pos
    // FIXME also doc comment wrong
    margin
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
            vec2(BOX_WIDTH, BOX_HEIGHT),
            &text,
        );
    }

    for (_addr, node) in &state.nodes {
        for link in &node.links {
            if link.dest_global {
                continue;
            }
            let src = match link.link_type {
                // TODO consts
                LinkType::Attrib => node.pos + vec2(0.0, 200.0),
                LinkType::Payload(i) => {
                    let offset_x = 10.0 + 300.0 * i as f32;
                    node.pos + vec2(offset_x, BOX_HEIGHT)
                }
            };
            let dest = state.nodes[&link.dest_addr].pos;
            draw_connection(state, src, dest);
        }
    }
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

fn draw_connection(state: &State, src: Vec2, dest: Vec2) {
    let src = src + state.offset;
    let dest = dest + state.offset;

    // Direct - for debugging
    draw_line(src.x, src.y, dest.x, dest.y, 1.0, BLUE);

    // Do some sane routing - TODO
    //draw_line(src.x, src.y, src.x, dest.y, 1.0, GREEN);
    //draw_line(src.x, dest.y, dest.x, dest.y, 1.0, GREEN);
}
