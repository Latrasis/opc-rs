```
  ___  _ __   ___      _ __ ___
 / _ \| '_ \ / __|____| '__/ __|
| (_) | |_) | (_|_____| |  \__ \
 \___/| .__/ \___|    |_|  |___/
      |_|
```
## OPC-RS
A rust implementation of the open pixel control protocol

## Open Pixel control
Open Pixel Control is a protocol that is used to control arrays of RGB lights like Total Control Lighting (http://www.coolneon.com/) and Fadecandy devices (https://github.com/scanlime/fadecandy).

## Documentation

TODO

## Usage:

### Client:

```rust
extern crate opc;

use std::net::TcpStream;
use opc::*;

fn main() {

    // Connect to a TCP Socket
    let mut stream = TcpStream::connect("127.0.0.1:7890").unwrap();
    // Create a Client
    let mut client = Client::new(stream);

    // Create a Vector of Pixels
    let mut pixels = vec![[0,0,0]; 1000];

    // Create Message
    let pixel_msg = Message {
      channel: 1,
      command: Command::SetPixelColors { pixels: pixels }
    };

    // Send Message
    client.send(pixel_msg);
}

```
