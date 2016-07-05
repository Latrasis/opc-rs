
extern crate opc;

use std::io::prelude::*;
use std::net::TcpStream;
use opc::*;

fn main() {

    let pixels = [[222,100,0]; 50];
    let pixel_msg = Message {
        channel: 0,
        command: Command::SetPixelColors { pixels: &pixels }
    };

    {
        let mut stream = TcpStream::connect("127.0.0.1:7890").unwrap();

        // ignore the Result
        let _ = stream.write_all(&pixel_msg.serialize());
    }
    println!("{:?}", pixel_msg.serialize());
}
