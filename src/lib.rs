extern crate byteorder;

use byteorder::{ByteOrder, BigEndian};

pub const MAX_OPC_DATA_SIZE: usize = 65536;
pub const MAX_OPC_PIXELS_SIZE: usize = 21845;

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
        self.channel == 0
    }

    pub fn serialize(&self) -> Vec<u8> {

        let mut ser_len_buf = [0u8, 2];
        let mut ser_len = self.len();
        BigEndian::write_u16(&mut ser_len_buf, ser_len as u16);

        match self.command {
            Command::SetPixelColors {ref pixels} => {
                let mut ser_data: Vec<u8> = pixels
                .to_owned()
                .iter()
                .flat_map(|rgb| rgb.into_iter())
                .cloned()
                .collect();

                // Insert Data Length
                ser_data.insert(0, ser_len_buf[1]);
                ser_data.insert(0, ser_len_buf[0]);
                // Insert Command
                ser_data.insert(0, 0);
                // Insert Channel
                ser_data.insert(0, self.channel);

                ser_data
            },
            Command::SystemExclusive {id, ref data} => {
                let mut ser_data = id.to_vec();
                ser_data.extend(data.iter());

                // Insert Data Length
                ser_data.insert(0, ser_len_buf[1]);
                ser_data.insert(0, ser_len_buf[0]);
                // Insert Command
                ser_data.insert(0, 255);
                // Insert Channel
                ser_data.insert(0, self.channel);

                ser_data
            }
        }
    }
}
