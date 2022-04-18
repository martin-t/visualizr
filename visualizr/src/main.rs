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
    offset: Vec2,
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
            dbg!(&update);
            state = Some(State {
                globals: update.globals,
                sexprecs: update.sexprecs,
                offset: Vec2::ZERO,
            });
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

fn draw_initial_box() {
    draw_box(
        0,
        vec2(100.0, 100.0),
        vec2(500.0, 50.0),
        "<waiting for input from visualizr>",
    );
}

fn draw_tree(state: &mut State) {
    let mut incoming_ptrs = HashMap::new();
    for sexprec in &state.sexprecs {
        let addr = state.globals.fmt_ptr(sexprec.address);
        incoming_ptrs.entry(addr).or_insert(0);

        let mut ptrs = sexprec.payload.pointers();
        ptrs.push(("", sexprec.attrib));
        for (_name, ptr) in ptrs {
            let addr = state.globals.fmt_ptr(ptr);
            dbg!(_name, &addr);
            let cnt = incoming_ptrs.entry(addr).or_insert(0);
            *cnt += 1;
        }
    }
    dbg!(incoming_ptrs);

    let box_size = vec2(950.0, 290.0);
    let mut box_pos = vec2(50.0, 50.0);
    let delta_y = 400.0;

    for sexprec in &state.sexprecs {
        let text = SexpFormatter(&state.globals, sexprec).to_string();
        draw_box(sexprec.address.0, box_pos, box_size, &text);
        box_pos.y += delta_y;
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
