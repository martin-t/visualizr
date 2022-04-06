use std::{
    collections::VecDeque,
    io::ErrorKind,
    net::{TcpListener, TcpStream},
};

use commonr::{data::SexprecHeader, net};
use macroquad::{
    hash,
    prelude::*,
    ui::{root_ui, widgets::Group},
};

#[derive(Debug)]
struct Server {
    listener: TcpListener,
    connection: Option<Connection>,
    msgs: Vec<SexprecHeader>,
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

#[macroquad::main("visualizr")]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:26000").unwrap();
    listener.set_nonblocking(true).unwrap();

    let mut server = Server {
        listener,
        connection: None,
        msgs: Vec::new(),
    };

    let mut text = "messages:\n".to_owned();
    loop {
        server.receive();

        for msg in server.msgs.drain(..) {
            dbg!(&msg);
            text.push_str(
                "sxpinfo type scalar obj alt gp mark debug trace spare gcgen gccls named extra\n",
            );
            // TODO gp wider and as bits
            text.push_str(&format!(
                "sxpinfo {:4} {:6} {:3} {:3} {:2} {:4} {:5} {:5} {:5} {:5} {:5} {:5} {:5}\n",
                msg.sxpinfo.ty,
                msg.sxpinfo.scalar,
                msg.sxpinfo.obj,
                msg.sxpinfo.alt,
                msg.sxpinfo.gp,
                msg.sxpinfo.mark,
                msg.sxpinfo.debug,
                msg.sxpinfo.trace,
                msg.sxpinfo.spare,
                msg.sxpinfo.gcgen,
                msg.sxpinfo.gccls,
                msg.sxpinfo.named,
                msg.sxpinfo.extra,
            ));
            text.push_str(&format!("sxpinfo as bits {:#b}\n", msg.sxpinfo_bits));
            text.push_str(&format!("attrib {:#x}\n", msg.attrib));
            text.push_str(&format!("gengc_next_node {:#x}\n", msg.gengc_next_node));
            text.push_str(&format!("gengc_prev_node {:#x}\n", msg.gengc_prev_node));
            text.push('\n');
        }

        clear_background(WHITE);

        let box1_pos = vec2(50.0, 50.0);
        let box2_pos = vec2(100.0, 400.0);
        let box_size = vec2(800.0, 250.0);

        draw_box(1, box1_pos, box_size, &text);
        draw_box(2, box2_pos, box_size, &text);

        let offset = 20.0;
        draw_connection(
            box1_pos + vec2(offset, box_size.y),
            box2_pos + vec2(0.0, offset),
        );

        next_frame().await
    }
}

fn draw_box(id: u64, box_pos: Vec2, box_size: Vec2, text: &str) {
    // We wanna allow copying the data (especially stuff like pointers) but not editing
    // so we use an InputText but reset the text every frame.
    // There seems to be no native way to allow copying from a Label
    // or disable editing in an InputText.
    // TODO maybe use Editobx + multiline?

    Group::new(hash!() + id, box_size)
        .position(box_pos)
        .ui(&mut root_ui(), |ui| {
            for line in text.split('\n') {
                let mut line = line.to_owned();
                ui.input_text(hash!() + id, "", &mut line);
            }
        });
}

fn draw_connection(src: Vec2, dest: Vec2) {
    draw_line(src.x, src.y, src.x, dest.y, 1.0, GREEN);
    draw_line(src.x, dest.y, dest.x, dest.y, 1.0, GREEN);
}
