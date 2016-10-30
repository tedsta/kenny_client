extern crate termion;

use std::io::{Read, Write, Stdout, Stdin, stdout};
use std::mem;
use std::net::UdpSocket;
use std::sync::mpsc::channel;
use std::thread;

use termion::{AsyncReader, async_stdin, color, cursor, style};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

#[derive(Copy, Clone, PartialEq)]
pub enum MoveDirection {
    Stop,
    Up,
    Down,
    Left,
    Right,
}

impl MoveDirection {
    pub fn to_u8(self) -> u8 {
        use MoveDirection::*;
        match self {
            Stop => 0,
            Up => 1,
            Down => 2,
            Left => 3,
            Right => 4,
        }
    }
}

struct KennyControl {
    stdin: AsyncReader,
    stdout: RawTerminal<Stdout>,
    client: UdpSocket,
    server_address: (String, u16),
    running: bool,
    dirty: bool,
    direction: MoveDirection,
}

impl KennyControl {
    pub fn new(ip_address: String, port: u16) -> KennyControl {
        let mut stdin: AsyncReader = async_stdin();
        let stdout: RawTerminal<Stdout> = stdout().into_raw_mode().unwrap();

        let server_address = (ip_address, port);

        // Create a UDP socket to talk to the rover
        let client = UdpSocket::bind("0.0.0.0:20002").unwrap();
        client.send_to(b"connect me plz", (server_address.0.as_ref(), server_address.1)).unwrap();

        let client_in = client.try_clone().unwrap();
        let (packet_t, packet_r) = channel();

        thread::Builder::new()
            .name("packet_in".to_string())
            .spawn(move || {
                let mut buf = [0u8; 512];
                loop {
                    let (bytes_read, _) = client_in.recv_from(&mut buf).unwrap();
                    //let bytes_read = client_in.read(&mut buf).unwrap();
                    if let Ok(msg) = String::from_utf8(buf[0..bytes_read].iter().cloned().collect()) {
                        packet_t.send(msg).unwrap();
                    }
                }
            }).unwrap();

        KennyControl {
            stdin: stdin,
            stdout: stdout,
            client: client,
            server_address: server_address,
            running: true,
            dirty: true,
            direction: MoveDirection::Stop,
        }
    }

    pub fn run(&mut self) {
        while self.running {
            self.handle_events();
            // TODO: Handle packets
            if self.dirty {
                // Redraw screen if it's dirty
                self.present();
            }
        }
    }

    fn present(&mut self) {
        // Clear dirty flag
        self.dirty = false;

        // Clear the screen
        write!(self.stdout, "{}", termion::clear::All);

        // Draw main text
        write!(self.stdout, "{}{}{}{}{}",
               cursor::Goto(1, 1),
               style::Bold, color::Fg(color::White),
               color::Bg(color::Reset), "Press ~ to exit");

        let dir_string = match self.direction {
            MoveDirection::Up => "Up",
            MoveDirection::Down => "Down",
            MoveDirection::Left => "Left",
            MoveDirection::Right => "Right",
            MoveDirection::Stop => "Stop",
        };
        write!(self.stdout, "{}{}{}{}{}",
               cursor::Goto(1, 2),
               style::Bold, color::Fg(color::White),
               color::Bg(color::Reset), dir_string);

        self.stdout.flush().unwrap();
    }

    fn handle_events(&mut self) {
        use std::io::Read;
        use termion::event;

        let mut bytes = [0u8; 64];
        let bytes_read = self.stdin.read(&mut bytes).unwrap();
        let ref mut bytes = bytes[..bytes_read].iter().map(|b| Ok(*b));
        while let Some(b) = bytes.next() {
            let e = match event::parse_event(b, bytes) {
                Ok(e) => e,
                Err(e) => {
                    println!("ERROR: failed to parse event: {:?}", e);
                    return;
                },
            };
            let k = if let event::Event::Key(k) = e { k } else { continue; };
            match k {
                Key::Char('\n') => { },
                Key::Char('~') => {
                    self.running = false;
                },
                Key::Char('w') => {
                    // Up
                    if self.direction == MoveDirection::Up {
                        self.direction = MoveDirection::Stop;
                    } else {
                        self.direction = MoveDirection::Up;
                    }
                    self.dirty = true;
                    self.send_direction();
                },
                Key::Char('s') => {
                    // Down
                    if self.direction == MoveDirection::Down {
                        self.direction = MoveDirection::Stop;
                    } else {
                        self.direction = MoveDirection::Down;
                    }
                    self.dirty = true;
                    self.send_direction();
                },
                Key::Char('a') => {
                    // Left
                    if self.direction == MoveDirection::Left {
                        self.direction = MoveDirection::Stop;
                    } else {
                        self.direction = MoveDirection::Left;
                    }
                    self.dirty = true;
                    self.send_direction();
                },
                Key::Char('d') => {
                    // Right
                    if self.direction == MoveDirection::Right {
                        self.direction = MoveDirection::Stop;
                    } else {
                        self.direction = MoveDirection::Right;
                    }
                    self.dirty = true;
                    self.send_direction();
                },
                Key::Char(c) => {
                },
                Key::Backspace => { },
                _ => { continue; },
            };
        }
    }

    pub fn send_direction(&self) {
        let dir = self.direction.to_u8();
        self.client.send_to(&[dir], (self.server_address.0.as_ref(), self.server_address.1));
    }
}

fn main() {
    let mut kenny = KennyControl::new("10.1.10.39".to_string(), 20001);
    kenny.run();
}
