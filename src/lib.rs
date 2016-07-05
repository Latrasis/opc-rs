extern crate byteorder;

use byteorder::{BigEndian, WriteBytesExt};
use std::io::*;
use std::borrow::Cow;

pub const MAX_OPC_DATA_SIZE: usize = 65536;
pub const MAX_OPC_PIXELS_SIZE: usize = 21845;

pub const COMMAND_PIXEL_COLORS: u8 = 0;
pub const COMMAND_SYS_EXCLUSIVE: u8 = 255;
pub const BROADCAST_CHANNEL: u8 = 0;

pub enum Command<'data> {
    SetPixelColors { pixels: & 'data [[u8; 3]] },
    SystemExclusive { id: [u8; 2], data: & 'data [u8] }
}

pub struct Message<'data> {
    pub channel: u8,
    pub command: Command<'data>
}

impl<'data> Message<'data> {
    pub fn new (ch: u8, cmd: Command<'data>) -> Message {
        Message {
            channel: ch,
            command: cmd
        }
    }

    /// Check Message Data Length
    fn len(&self) -> usize {
        match self.command {
            Command::SetPixelColors {ref pixels} => pixels.len()*3,
            Command::SystemExclusive {id, ref data} => data.len() + 2
        }
    }

    /// Check is Message has a valid size
    pub fn is_valid(&self) -> bool {
        self.len() <= MAX_OPC_DATA_SIZE
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
        Client { writer: BufWriter::with_capacity(MAX_OPC_DATA_SIZE, writer)}
    }

    pub fn send(&mut self, msg: Message) -> Result<()> {

        let ser_len = msg.len();

        match msg.command {
            Command::SetPixelColors {pixels} => {

                // Insert Channel and Command
                try!(self.writer.write(&[msg.channel, COMMAND_PIXEL_COLORS]));
                // Insert Data Length
                self.writer.write_u16::<BigEndian>(ser_len as u16);

                // Insert Data
                for pixel in pixels {
                    self.writer.write(pixel);
                }
            },
            Command::SystemExclusive {id, data} => {

                // Insert Channel and Command
                try!(self.writer.write(&[msg.channel, COMMAND_SYS_EXCLUSIVE]));
                // Insert Data Length
                self.writer.write_u16::<BigEndian>(ser_len as u16);

                // Insert Data
                try!(self.writer.write(&id));
                try!(self.writer.write(&data));
            }
        }

        self.writer.flush()
    }
}
