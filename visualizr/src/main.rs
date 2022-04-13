use std::{
    collections::VecDeque,
    io::ErrorKind,
    net::{TcpListener, TcpStream},
};

use commonr::{data::Update, net};
use macroquad::{
    hash,
    prelude::*,
    ui::{root_ui, widgets::Group},
};

#[derive(Debug)]
struct Server {
    listener: TcpListener,
    connection: Option<Connection>,
    msgs: Vec<Update>,
}

impl Server {
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
    let listener = TcpListener::bind("127.0.0.1:26000").unwrap();
    listener.set_nonblocking(true).unwrap();

    let mut server = Server {
        listener,
        connection: None,
        msgs: Vec::new(),
    };

    let mut text = "<nothing>\n".to_owned();
    loop {
        server.receive();

        for msg in server.msgs.drain(..) {
            dbg!(&msg);
            text = msg.to_string();
        }

        clear_background(WHITE);

        let box1_pos = vec2(50.0, 50.0);
        let box2_pos = vec2(100.0, 600.0);
        let box_size = vec2(950.0, 500.0);

        draw_box(1, box1_pos, box_size, &text);
        draw_box(2, box2_pos, box_size, &text);

        let offset = 20.0;
        draw_connection(
            box1_pos + vec2(offset, box_size.y),
            box2_pos + vec2(0.0, offset),
        );

        // let mut single_line = "test".to_owned();
        // Group::new(hash!(), box_size)
        //     .position(box1_pos)
        //     .ui(&mut root_ui(), |ui| {
        //         ui.input_text(hash!(), "", &mut single_line);
        //     });

        next_frame().await
    }
}

fn draw_box(id: u64, box_pos: Vec2, box_size: Vec2, text: &str) {
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
