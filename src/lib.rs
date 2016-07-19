extern crate byteorder;

#[allow (unused_variables)]
#[allow (dead_code)]
use byteorder::{BigEndian, ByteOrder, WriteBytesExt};
use std::io::*;

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
        pixels: Vec<[u8; 3]>
    },
    /// Used to send a message that is specific to a particular device or software system.
    SystemExclusive {
        /// The data block should begin with a two-byte system ID.
        id: [u8; 2],
        /// designers of that system are then free to define any message format for the rest of the data block.
        data: Vec<u8> }
}

/// Describes a single message that follows the OPC protocol
#[derive (Clone, Debug, PartialEq)]
pub struct Message {
    /// Up to 255 separate strands of pixels can be controlled.
    /// Channel 0 are considered broadcast messages.
    /// Channels number from 1 to 255 are for each strand and listen for messages with that channel number.
    pub channel: u8,
    /// Designates the message type
    pub command: Command
}

impl Message {
    /// Create new Message Instance
    pub fn new (ch: u8, cmd: Command) -> Message {
        Message {
            channel: ch,
            command: cmd
        }
    }

    /// Check Message Data Length
    pub fn len(&self) -> usize {
        match self.command {
            Command::SetPixelColors {ref pixels} => pixels.len()*3,
            Command::SystemExclusive {id: _, ref data} => data.len() + 2
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
                try!(self.writer.write(&[msg.channel, SET_PIXEL_COLORS]));
                // Insert Data Length
                try!(self.writer.write_u16::<BigEndian>(ser_len as u16));

                // Insert Data
                for pixel in pixels {
                    try!(self.writer.write(&pixel));
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


pub trait Device {
    fn write_msg(&mut self, msg: &Message) -> Result<()>;
    fn channel(&self) -> u8;
}

pub struct Server<R: Read, D: Device> {
    reader: BufReader<R>,
    devices: Vec<D>
}

impl <R: Read, D: Device> Server<R, D> {
    pub fn new(reader: R) -> Server<R, D> {
        Server {
            reader: BufReader::with_capacity(MAX_MESSAGE_SIZE, reader),
            devices: Vec::<D>::default()
        }
    }

    pub fn register(&mut self, device: D) -> Result<()> {
        Ok(self.devices.push(device))
    }

    fn dispatch(msg: &Message, devices: &mut [D]) -> Result<()>{
        devices.iter_mut()
            .filter(|device| msg.channel == 0 || device.channel() == msg.channel)
            .map(|device| device.write_msg(msg))
            .find(|res| res.is_err())
            .unwrap_or(Ok(()))
    }

    fn read_msg(&mut self) -> Result<Message> {

        let (msg, length) = {
            let buf = try!(self.reader.fill_buf());

            // Check if buf length is more than 4;
            if buf.len() < 4 {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid Message Command"));
            };

            let (channel, command) = (buf[0], buf[1]);
            let length = BigEndian::read_u16(&buf[2..4]) as usize;
            let data = &buf[4..][..length];
            let msg = match command {
                SET_PIXEL_COLORS => {
                    let pixels: Vec<_> = data[..(length-(length % 3))].chunks(3).map(|chunk| [chunk[0],chunk[1],chunk[2]]).collect();
                    Message {
                        channel: channel,
                        command: Command::SetPixelColors { pixels: pixels }
                    }

                },
                SYS_EXCLUSIVE => {
                    Message {
                        channel: channel,
                        command: Command::SystemExclusive { id: [data[0], data[1]], data: data[2..].to_vec() }
                    }
                },
                // TODO: What to do if incorrect?
                _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid Message Command"))
            };

            (msg, length+4)
        };

        self.reader.consume(length);
        Ok(msg)
    }

    pub fn process(&mut self) -> Result<()> {
        while let Ok(ref msg) = self.read_msg() {
            try!(Self::dispatch(msg, self.devices.as_mut()));
        }
        Ok(())
    }

}

#[test]
fn server_should_receive_pixel_command() {

    let mut client = Client::new(Vec::new());
    let test_msg = Message {
        channel: 4,
        command: Command::SetPixelColors { pixels: vec![[9; 3]; 10]}
    };

    client.send(test_msg.clone());

    struct TestDevice;
    impl Device for TestDevice {
        fn write_msg(&mut self, msg: &Message) -> Result<()> {
            assert_eq!(&Message {
                channel: 4,
                command: Command::SetPixelColors { pixels: vec![[9; 3]; 10]}
            }, msg);
            Ok(())
        }
        fn channel(&self) -> u8 { 4 }
    }

    let mut server = Server::new(client.writer.get_ref().as_slice());
    server.register(TestDevice {});
    server.process();
}

#[test]
fn server_should_receive_system_command() {

    let mut client = Client::new(Vec::new());
    let test_msg = Message {
        channel: 4,
        command: Command::SystemExclusive { id: [0; 2], data: vec![8; 10]}
    };

    client.send(test_msg.clone());

    struct TestDevice;
    impl Device for TestDevice {
        fn write_msg(&mut self, msg: &Message) -> Result<()> {
            assert_eq!(&Message {
                channel: 4,
                command: Command::SystemExclusive { id: [0; 2], data: vec![8; 10]}
            }, msg);
            Ok(())
        }
        fn channel(&self) -> u8 { 4 }
    }

    let mut server = Server::new(client.writer.get_ref().as_slice());
    server.register(TestDevice {});
    server.process();

}
