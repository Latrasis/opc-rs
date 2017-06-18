extern crate opc;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

use opc::OpcCodec;
use futures::{Future, Stream};

use tokio_io::AsyncRead;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let remote_addr = "127.0.0.1:7890".parse().unwrap();

    let listener = TcpListener::bind(&remote_addr, &handle).unwrap();

    // Accept all incoming sockets
    let server = listener.incoming().for_each(move |(socket, _)| {
        // `OpcCodec` handles encoding / decoding frames.
        let transport = socket.framed(OpcCodec);

        let process_connection = transport.for_each(|message| {
            println!("GOT: {:?}", message);
            Ok(())
        });

        // Spawn a new task dedicated to processing the connection
        handle.spawn(process_connection.map_err(|_| ()));

        Ok(())
    });

    // Open listener
    core.run(server).unwrap();
}
