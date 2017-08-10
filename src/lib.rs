//!     
//!     #Open Pixel Control
//!     
//!     Open Pixel Control is a protocol that is used to control arrays of RGB lights
//!     like [Total Control Lighting](http://www.coolneon.com/) and [Fadecandy devices](https://github.com/scanlime/fadecandy).
//!     
//!     
//!     # Examples
//!     Setup Server to Listen for Messages: 
//!     
//!     ```rust,no_run
//!     extern crate opc;
//!     extern crate futures;
//!     extern crate tokio_core;
//!     extern crate tokio_io;
//!     
//!     use opc::OpcCodec;
//!     use futures::{Future, Stream};
//!     
//!     use tokio_io::AsyncRead;
//!     use tokio_core::net::TcpListener;
//!     use tokio_core::reactor::Core;
//!     
//!     fn main() {
//!         let mut core = Core::new().unwrap();
//!         let handle = core.handle();
//!         let remote_addr = "127.0.0.1:7890".parse().unwrap();
//!     
//!         let listener = TcpListener::bind(&remote_addr, &handle).unwrap();
//!     
//!         // Accept all incoming sockets
//!         let server = listener.incoming().for_each(move |(socket, _)| {
//!             // `OpcCodec` handles encoding / decoding frames.
//!             let transport = socket.framed(OpcCodec);
//!     
//!             let process_connection = transport.for_each(|message| {
//!                 println!("GOT: {:?}", message);
//!                 Ok(())
//!             });
//!     
//!             // Spawn a new task dedicated to processing the connection
//!             handle.spawn(process_connection.map_err(|_| ()));
//!     
//!             Ok(())
//!         });
//!     
//!         // Open listener
//!         core.run(server).unwrap();
//!     }
//!     ```

extern crate tokio_io;
extern crate bytes;

use std::io;

use tokio_io::codec::{Encoder, Decoder};
use bytes::{BytesMut, BufMut, Buf, IntoBuf, BigEndian};

/// Default openpixel tcp port
pub const DEFAULT_OPC_PORT: usize = 7890;

const MAX_MESSAGE_SIZE: usize = 0xffff;
const SYS_EXCLUSIVE: u8 = 0xff;
const SET_PIXEL_COLORS: u8 = 0x00;
const BROADCAST_CHANNEL: u8 = 0;

/// Describes an OPC Command.
#[derive (Clone, Debug, PartialEq)]
pub enum Command {
    /// Contains and array of RGB values: three bytes in red, green, blue order for each pixel to set.
    SetPixelColors {
        /// If the data block has length 3*n, then the first n pixels of the specified channel are set.
        /// All other pixels are unaffected and retain their current colour values.
        /// If the data length is not a multiple of 3, or there is data for more pixels than are present, the extra data is ignored.
        pixels: Vec<[u8; 3]>,
    },
    /// Used to send a message that is specific to a particular device or software system.
    SystemExclusive {
        /// The data block should begin with a two-byte system ID.
        id: [u8; 2],
        /// designers of that system are then free to define any message format for the rest of the data block.
        data: Vec<u8>,
    },
}

/// Describes a single message that follows the OPC protocol
#[derive (Clone, Debug, PartialEq)]
pub struct Message {
    /// Up to 255 separate strands of pixels can be controlled.
    /// Channel 0 are considered broadcast messages.
    /// Channels number from 1 to 255 are for each strand and listen for messages with that channel number.
    pub channel: u8,
    /// Designates the message type
    pub command: Command,
}

impl Message {
    /// Create new Message Instance from Pixel Array
    pub fn from_pixels(ch: u8, pixels: &[[u8; 3]]) -> Message {
        Message {
            channel: ch,
            command: Command::SetPixelColors { pixels: pixels.to_owned() },
        }
    }

    /// Create new Message Instance from Data Array
    pub fn from_data(ch: u8, id: &[u8; 2], data: &[u8]) -> Message {
        Message {
            channel: ch,
            command: Command::SystemExclusive {
                id: id.to_owned(),
                data: data.to_owned(),
            },
        }
    }

    /// Check Message Data Length
    pub fn len(&self) -> usize {
        match self.command {
            Command::SetPixelColors { ref pixels } => pixels.len() * 3,
            Command::SystemExclusive { id: _, ref data } => data.len() + 2,
        }
    }

    /// Check is Message has a valid size
    pub fn is_valid(&self) -> bool {
        self.len() <= MAX_MESSAGE_SIZE
    }

    /// Check if Message is a broadcast message
    pub fn is_broadcast(&self) -> bool {
        self.channel == BROADCAST_CHANNEL
    }
}

/// Open Pixel Codec Instance
pub struct OpcCodec;

impl Decoder for OpcCodec {
    type Item = Message;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Self::Item>> {

        let (msg, length) = {
            // Get Temporary Src
            let mut src = src.clone();

            // Check if buf length is more than 4;
            if src.len() < 4 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Message Command"));
            };
            let mut buf = src.split_to(4).into_buf();

            let (channel, command) = (buf.get_u8(), buf.get_u8());
            let length = buf.get_u16::<BigEndian>() as usize;

            if src.len() < length {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Message Command"));
            }
            let mut buf = src.split_to(length).into_buf();

            let msg = match command {
                SET_PIXEL_COLORS => {
                    let pixels: Vec<_> = buf.bytes()[..length - (length % 3)]
                        .chunks(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                        .collect();
                    Message {
                        channel: channel,
                        command: Command::SetPixelColors { pixels: pixels },
                    }
                }
                SYS_EXCLUSIVE => {
                    Message {
                        channel: channel,
                        command: Command::SystemExclusive {
                            id: [buf.get_u8(), buf.get_u8()],
                            data: buf.collect(),
                        },
                    }
                }
                // TODO: What to do if incorrect?
                _ => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                                              "Invalid Message Command"))
                }
            };

            (msg, length + 4)
        };

        // Advance
        src.split_to(length);
        Ok(Some(msg))
    }
}

impl Encoder for OpcCodec {
    type Item = Message;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, dst: &mut BytesMut) -> io::Result<()> {

        let ser_len = msg.len();
        dst.reserve(4 + ser_len);

        match msg.command {
            Command::SetPixelColors { pixels } => {

                // Insert Channel and Command
                dst.put_slice(&[msg.channel, SET_PIXEL_COLORS]);
                // Insert Data Length
                dst.put_u16::<BigEndian>(ser_len as u16);

                // Insert Data
                for pixel in pixels {
                    dst.put_slice(&pixel);
                }
            }
            Command::SystemExclusive { id, data } => {

                // Insert Channel and Command
                dst.put_slice(&[msg.channel, SYS_EXCLUSIVE]);
                // Insert Data Length
                dst.put_u16::<BigEndian>(ser_len as u16);

                // Insert Data
                dst.put_slice(&id);
                dst.put_slice(&data);
            }
        }

        Ok(())
    }
}

#[test]
fn should_roundtrip_pixel_command() {

    let mut codec = OpcCodec;
    let mut buf = BytesMut::new();
    let test_msg = Message {
        channel: 4,
        command: Command::SetPixelColors { pixels: vec![[9; 3]; 10] },
    };

    assert!(codec.encode(test_msg.clone(), &mut buf).is_ok());

    let recv_msg = codec.decode(&mut buf.into()).unwrap().unwrap();

    assert_eq!(test_msg, recv_msg);

}

#[test]
fn server_roundtrip_system_command() {

    let mut codec = OpcCodec;
    let mut buf = BytesMut::new();
    let test_msg = Message {
        channel: 4,
        command: Command::SystemExclusive {
            id: [0; 2],
            data: vec![8; 10],
        },
    };

    assert!(codec.encode(test_msg.clone(), &mut buf).is_ok());

    let recv_msg = codec.decode(&mut buf.into()).unwrap().unwrap();

    assert_eq!(test_msg, recv_msg);

}
