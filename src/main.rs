
extern crate opc;

use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration};
use std::cell::Cell;
use opc::*;

fn main() {

    let mut stream = TcpStream::connect("127.0.0.1:7890").unwrap();
    let mut client = Client::new(stream);

    let child = thread::spawn(move || {
        let mut pixels = [[0,255,0]; 500];

        for i in 0..pixels.len() {

            thread::sleep(Duration::from_millis(10));
            pixels[i][2] = 255;

            let pixel_msg = Message {
                channel: 1,
                command: Command::SetPixelColors { pixels: &pixels }
            };
            client.send(pixel_msg);
        }
    });

    let res = child.join();
}
