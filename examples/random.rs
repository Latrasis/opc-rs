extern crate opc;
extern crate rand;

use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration};
use std::cell::Cell;
use opc::*;
use rand::Rng;


fn main() {

    let mut stream = TcpStream::connect("127.0.0.1:7890").unwrap();
    let mut client = Client::new(stream);

    let child = thread::spawn(move || {
        let mut pixels: [[u8; 3]; 1000] = [[0,0,0]; 1000];
        let mut rng = rand::thread_rng();

        loop {

            for pixel in pixels.iter_mut() {
                for c in 0..2 {
                    pixel[c] = rng.gen();
                }
            }

            let pixel_msg = Message {
                channel: 1,
                command: Command::SetPixelColors { pixels: &pixels }
            };
            client.send(pixel_msg);
            thread::sleep(Duration::from_millis(1000));
        }
    });

    let res = child.join();
}
