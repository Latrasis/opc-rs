extern crate opc;
extern crate tokio_core;
extern crate futures;
extern crate rand;

use opc::{OpcCodec, Message, Command};
use futures::{stream, Future, Stream, Sink, future};

use tokio_core::io::Io;
use tokio_core::net::{TcpStream, TcpListener};
use tokio_core::reactor::Core;

use std::{io, thread};
use std::time::Duration;

use rand::*;

fn main() {

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let remote_addr = "192.168.1.230:7890".parse().unwrap();

    let work = TcpStream::connect(&remote_addr, &handle)
        .and_then(|socket| {
            let transport = socket.framed(OpcCodec);

            let messages = stream::unfold(vec![[0,0,0]; 1000], |mut pixels| {

                for pixel in pixels.iter_mut() {
                    for c in 0..2 {
                        pixel[c] = rand::random();
                    }
                };

                let pixel_msg = Message {
                    channel: 0,
                    command: Command::SetPixelColors { pixels: pixels.clone() }
                };

                std::thread::sleep(Duration::from_millis(100));

                Some(future::ok::<_,io::Error>((pixel_msg, pixels)))
            });

            transport.send_all(messages)

        });

    core.run(work).unwrap();
}
