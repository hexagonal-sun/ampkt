use anyhow::Result;
use futuresdr::{
    async_trait::async_trait,
    blocks::Apply,
    num_complex::Complex32,
    runtime::{
        Block, BlockMeta, BlockMetaBuilder, Kernel, MessageIo, MessageIoBuilder, StreamIo,
        StreamIoBuilder, WorkIo,
    },
};

use crate::sym::{Sym, Symbol};

pub struct QamMod {
    sps: u16,
}

impl QamMod {
    pub fn new(sps: u16) -> Block {
        Block::new(
            BlockMetaBuilder::new("QamMod").build(),
            StreamIoBuilder::new()
                .add_input("in", std::mem::size_of::<Symbol>())
                .add_output("out", std::mem::size_of::<Complex32>())
                .build(),
            MessageIoBuilder::new().build(),
            QamMod { sps },
        )
    }
}

#[async_trait]
impl Kernel for QamMod {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        let input = sio.input(0);
        let is = input.slice::<Symbol>();
        let out_output = sio.output(0);
        let os = out_output.slice::<Complex32>();
        let mut syms_processed = 0;

        if os.len() == 0 {
            return Ok(());
        }

        if sio.input(0).finished() {
            io.finished = true;
        }

        for (i, chunk) in os.chunks_exact_mut(self.sps as usize).enumerate() {
            if let Some(sym) = is.get(i) {
                let samp = if let Some(samp) = sym {
                    Complex32::from(samp)
                } else {
                    Complex32::new(0.0, 0.0)
                };

                for o in chunk.iter_mut() {
                    *o = samp
                }

                syms_processed = i;
            }
        }

        sio.input(0).consume(syms_processed);
        sio.output(0).produce(syms_processed * self.sps as usize);

        Ok(())
    }
}

pub struct QamDemod;

impl QamDemod {
    pub fn new() -> Block {
        Apply::new(move |x: &Complex32| {
            let pi = std::f32::consts::PI;
            let (mag, phase) = x.to_polar();

            if mag < 0.1 {
                return None;
            }

            if phase > 0.0 {
                if phase < pi / 2.0 {
                    Some(Sym::A)
                } else {
                    Some(Sym::B)
                }
            } else {
                if phase > -pi / 2.0 {
                    Some(Sym::C)
                } else {
                    Some(Sym::D)
                }
            }
        })
    }
}
