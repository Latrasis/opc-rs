extern crate byteorder;

use byteorder::{BigEndian, WriteBytesExt};
use std::io::*;

/// Default OPC Tcp Port
pub const DEFAULT_PORT: usize = 7890;

const MAX_MESSAGE_SIZE: usize = 0xffff;
const SYS_EXCLUSIVE: u8 = 0xff;
const PIXEL_COLORS: u8 = 0x00;
const BROADCAST_CHANNEL: u8 = 0;

/// Describes an OPC Command.
pub enum Command<'data> {
    /// Contains and array of RGB values: three bytes in red, green, blue order for each pixel to set.
    SetPixelColors {
        /// If the data block has length 3*n, then the first n pixels of the specified channel are set.
        /// All other pixels are unaffected and retain their current colour values.
        /// If the data length is not a multiple of 3, or there is data for more pixels than are present, the extra data is ignored.
        pixels: & 'data [[u8; 3]]
    },
    /// Used to send a message that is specific to a particular device or software system.
    SystemExclusive {
        /// The data block should begin with a two-byte system ID.
        id: [u8; 2],
        /// designers of that system are then free to define any message format for the rest of the data block.
        data: & 'data [u8] }
}

/// Describes a single message that follows the OPC protocol
pub struct Message<'data> {
    /// Up to 255 separate strands of pixels can be controlled.
    /// Channel 0 are considered broadcast messages.
    /// Channels number from 1 to 255 are for each strand and listen for messages with that channel number.
    pub channel: u8,
    /// Designates the message type
    pub command: Command<'data>
}

impl<'data> Message<'data> {
    /// Create new Message Instance
    pub fn new (ch: u8, cmd: Command<'data>) -> Message {
        Message {
            channel: ch,
            command: cmd
        }
    }

    /// Check Message Data Length
    pub fn len(&self) -> usize {
        match self.command {
            Command::SetPixelColors {ref pixels} => pixels.len()*3,
            Command::SystemExclusive {id, ref data} => data.len() + 2
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

pub struct Client<W: Write> {
    writer: BufWriter<W>
}

impl <W: Write> Client<W> {

    pub fn new(writer: W) -> Client<W> {
        Client { writer: BufWriter::with_capacity(MAX_MESSAGE_SIZE, writer)}
    }

    pub fn send(&mut self, msg: Message) -> Result<()> {

        let ser_len = msg.len();

        match msg.command {
            Command::SetPixelColors {pixels} => {

                // Insert Channel and Command
                try!(self.writer.write(&[msg.channel, PIXEL_COLORS]));
                // Insert Data Length
                try!(self.writer.write_u16::<BigEndian>(ser_len as u16));

                // Insert Data
                for pixel in pixels {
                    try!(self.writer.write(pixel));
                }
            },
            Command::SystemExclusive {id, data} => {

                // Insert Channel and Command
                try!(self.writer.write(&[msg.channel, SYS_EXCLUSIVE]));
                // Insert Data Length
                try!(self.writer.write_u16::<BigEndian>(ser_len as u16));

                // Insert Data
                try!(self.writer.write(&id));
                try!(self.writer.write(&data));
            }
        }

        self.writer.flush()
    }
}
