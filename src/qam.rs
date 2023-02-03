use std::{collections::VecDeque, io::Cursor};

use anyhow::Result;
use bitstream_io::{BigEndian, BitRead, BitReader};
use futuresdr::{
    async_trait::async_trait,
    macros::message_handler,
    num_complex::Complex32,
    runtime::{
        Block, BlockMeta, BlockMetaBuilder, Kernel, MessageIo, MessageIoBuilder, Pmt, StreamIo,
        StreamIoBuilder, StreamOutput, WorkIo,
    },
};

pub struct Qam {
    pkt_queue: VecDeque<Vec<u8>>,
    txing_pkt: Option<BitReader<Cursor<Vec<u8>>, BigEndian>>,
}

impl Qam {
    pub fn new() -> Block {
        Block::new(
            BlockMetaBuilder::new("Tap").build(),
            StreamIoBuilder::new()
                .add_output::<Complex32>("out")
                .build(),
            MessageIoBuilder::new()
                .add_input("in", Self::input_handler)
                .build(),
            Qam {
                pkt_queue: VecDeque::new(),
                txing_pkt: None,
            },
        )
    }

    #[message_handler]
    async fn input_handler(
        &mut self,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
        p: Pmt,
    ) -> Result<Pmt> {
        if let Pmt::Blob(data) = p {
            eprintln!("pkt pushed");
            self.pkt_queue.push_back(data);
        }

        Ok(Pmt::Null)
    }

    fn tx_bits(&mut self, output: &mut StreamOutput) {
        let o = output.slice();

        if let Some(ref mut br) = self.txing_pkt {
            let mut i = 0;

            while i < o.len() {
                if let Ok(bits) = br.read::<u8>(2) {
                    eprintln!("Writing pkt {i}");
                    o[i] = match bits {
                        0b00 => Complex32::new(1.0, 1.0),
                        0b01 => Complex32::new(-1.0, 1.0),
                        0b10 => Complex32::new(1.0, -1.0),
                        0b11 => Complex32::new(-1.0, -1.0),
                        _ => unreachable!(),
                    };
                } else {
                    self.txing_pkt = None;
                    eprintln!("PKT finished");
                    break;
                }

                i += 1;
            }

            output.produce(i);
        }
    }
}

#[async_trait]
impl Kernel for Qam {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        println!("working");
        let output = sio.output(0);

        if self.txing_pkt.is_some() {
            self.tx_bits(output);
            return Ok(());
        }

        if let Some(pkt) = self.pkt_queue.pop_front() {
            self.txing_pkt = Some(BitReader::endian(Cursor::new(pkt), BigEndian));
            self.tx_bits(output);
            return Ok(());
        }

        let out_buf = output.slice();

        for v in out_buf.iter_mut() {
            *v = Complex32::new(0.0, 0.0);
        }

        if self.txing_pkt.is_none() && self.pkt_queue.is_empty() && mio.input(0).finished() {
            io.finished = true;
        }

        Ok(())
    }
}
