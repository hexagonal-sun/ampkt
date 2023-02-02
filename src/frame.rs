use std::collections::VecDeque;

use anyhow::Result;
use futuresdr::{
    async_trait::async_trait,
    macros::message_handler,
    runtime::{
        Block, BlockMeta, BlockMetaBuilder, Kernel, MessageIo, MessageIoBuilder, Pmt, StreamIo,
        StreamIoBuilder, WorkIo,
    },
};

use crate::sym::{Sym, Symbol};
use crate::sym_sync::{SymSync, SYNC};

pub struct FrameEncoder {
    sym_queue: VecDeque<Symbol>,
}

impl FrameEncoder {
    pub fn new() -> Block {
        Block::new(
            BlockMetaBuilder::new("FrameEncoder").build(),
            StreamIoBuilder::new()
                .add_output("out", std::mem::size_of::<Symbol>())
                .build(),
            MessageIoBuilder::new()
                .add_input("in", Self::pkt_handler)
                .build(),
            Self::create(),
        )
    }

    fn create() -> Self {
        Self {
            sym_queue: VecDeque::new(),
        }
    }

    fn push_sync(&mut self) {
        self.sym_queue.extend(SYNC.iter().map(|x| Some(*x)));
    }

    fn push_sz(&mut self, len: u16) {
        self.push_byte((len >> 8) as u8);
        self.push_byte((len & 0xff) as u8);
    }

    fn push_byte(&mut self, byte: u8) {
        Sym::syms_from_byte(byte)
            .into_iter()
            .for_each(|s| self.sym_queue.push_back(Some(s)));
    }

    fn push_bytes(&mut self, bytes: &Vec<u8>) {
        bytes
            .iter()
            .flat_map(|byte| Sym::syms_from_byte(*byte))
            .for_each(|sym| self.sym_queue.push_back(Some(sym)))
    }

    fn push_frame(&mut self, bytes: &Vec<u8>) {
        self.push_sync();
        self.push_sync();
        self.push_sz(bytes.len() as u16);
        self.push_bytes(bytes);
    }

    #[message_handler]
    async fn pkt_handler(
        &mut self,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
        p: Pmt,
    ) -> Result<Pmt> {
        if let Pmt::Blob(ref data) = p {
            self.push_frame(data);
        }

        Ok(Pmt::Null)
    }
}

#[async_trait]
impl Kernel for FrameEncoder {
    async fn work(
        &mut self,
        _io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        let output = sio.output(0);
        let o: &mut [Symbol] = output.slice();

        if self.sym_queue.is_empty() {
            o.iter_mut().for_each(|x| *x = None);
            output.produce(o.len());
            return Ok(());
        }

        let mut iter = o.iter_mut().enumerate();

        for (_, o) in &mut iter {
            if let Some(sym) = self.sym_queue.pop_front() {
                *o = sym;
            } else {
                break;
            }
        }

        output.produce(if let Some((i, _)) = iter.next() {
            i - 1
        } else {
            o.len()
        });

        Ok(())
    }
}

struct ByteDecoder {
    cur_byte: u8,
    bits_pushed: u8,
}

impl ByteDecoder {
    fn new() -> Self {
        Self {
            cur_byte: 0,
            bits_pushed: 0,
        }
    }

    fn push_sym(&mut self, s: Sym) -> Option<u8> {
        self.cur_byte |= <Sym as Into<u8>>::into(s);
        self.bits_pushed += 2;

        if self.bits_pushed == 8 {
            let ret = self.cur_byte;
            self.reset();
            Some(ret)
        } else {
            self.cur_byte <<= 2;
            None
        }
    }

    fn reset(&mut self) {
        self.bits_pushed = 0;
        self.cur_byte = 0;
    }
}

struct U16Decoder {
    bd: ByteDecoder,
    high_byte: Option<u8>,
}

impl U16Decoder {
    fn new() -> Self {
        Self {
            bd: ByteDecoder::new(),
            high_byte: None,
        }
    }

    fn push_sym(&mut self, s: Sym) -> Option<u16> {
        if let Some(byte) = self.bd.push_sym(s) {
            if let Some(high_byte) = self.high_byte {
                self.high_byte = None;
                Some((high_byte as u16) << 8 | byte as u16)
            } else {
                self.high_byte = Some(byte);
                None
            }
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.high_byte = None;
        self.bd.reset();
    }
}

enum DecoderState {
    Sync,
    Sz,
    Data,
}

pub struct FrameDecoder {
    sym_sync: SymSync,
    state: DecoderState,
    frame_sz: u16,
    rotation: usize,
    frame_sz_decoder: U16Decoder,
    data: Vec<u8>,
    data_decoder: ByteDecoder,
}

impl FrameDecoder {
    pub fn new() -> Block {
        Block::new(
            BlockMetaBuilder::new("FrameDecoder").build(),
            StreamIoBuilder::new()
                .add_input("in", std::mem::size_of::<Symbol>())
                .build(),
            MessageIoBuilder::new().add_output("out").build(),
            Self::create(),
        )
    }

    fn create() -> Self {
        Self {
            sym_sync: SymSync::new(),
            state: DecoderState::Sync,
            rotation: 0,
            frame_sz: 0,
            frame_sz_decoder: U16Decoder::new(),
            data: Vec::new(),
            data_decoder: ByteDecoder::new(),
        }
    }

    fn push_sym(&mut self, s: Sym) -> Option<Vec<u8>> {
        if let Some(rotation) = self.sym_sync.push_sym(s) {
            self.rotation = rotation;
            self.reset();
            self.state = DecoderState::Sz;
            return None;
        }

        let s = s.sub(self.rotation);

        match self.state {
            DecoderState::Sync => None,
            DecoderState::Sz => {
                if let Some(sz) = self.frame_sz_decoder.push_sym(s) {
                    self.frame_sz = sz;
                    self.state = DecoderState::Data;
                }

                None
            }

            DecoderState::Data => {
                if let Some(byte) = self.data_decoder.push_sym(s) {
                    self.data.push(byte);

                    if self.data.len() == self.frame_sz as usize {
                        let data = self.data.clone();
                        self.reset();
                        return Some(data);
                    }
                }

                None
            }
        }
    }

    fn reset(&mut self) {
        self.state = DecoderState::Sync;
        self.data.clear();
        self.frame_sz = 0;
        self.frame_sz_decoder.reset();
        self.data_decoder.reset();
    }
}

#[async_trait]
impl Kernel for FrameDecoder {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        let input = sio.input(0).slice::<Symbol>();

        for samp in input.iter() {
            if samp.is_none() {
                continue;
            }

            if let Some(frame) = self.push_sym(samp.unwrap()) {
                mio.post(0, Pmt::Blob(frame)).await;
            }
        }

        if sio.input(0).finished() {
            mio.post(0, Pmt::Null).await;
            io.finished = true;
        }

        sio.input(0).consume(input.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::sym::Sym;

    use super::{FrameDecoder, FrameEncoder};

    fn run(mut sym_transform: impl FnMut(Sym) -> Sym) -> Result<()> {
        let mut encoder = FrameEncoder::create();
        let mut decoder = FrameDecoder::create();

        let payload = vec![0xde, 0xad, 0xbe, 0xef];

        encoder.push_frame(&payload);

        let mut it = encoder.sym_queue.iter().peekable();

        while let Some(sym) = it.next() {
            let v = decoder.push_sym(sym_transform(sym.unwrap()));

            if it.peek().is_none() {
                assert!(matches!(v, Some(_payload)));
            } else {
                assert!(v.is_none())
            }
        }

        Ok(())
    }

    #[test]
    fn encode_decode() -> Result<()> {
        run(|s| s)
    }

    #[test]
    fn encode_decode_rot_1() -> Result<()> {
        run(|s| s.add(1))
    }

    #[test]
    fn encode_decode_rot_2() -> Result<()> {
        run(|s| s.add(2))
    }

    #[test]
    fn encode_decode_rot_3() -> Result<()> {
        run(|s| s.add(3))
    }
}
