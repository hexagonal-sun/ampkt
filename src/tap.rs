use std::{io::ErrorKind, sync::Arc};

use anyhow::{bail, Result};
use futuresdr::{
    async_trait::async_trait,
    macros::message_handler,
    runtime::{
        Block, BlockMeta, BlockMetaBuilder, Kernel, MessageIo, MessageIoBuilder, Pmt, StreamIo,
        StreamIoBuilder, WorkIo,
    },
};
use tun_tap::Iface;

pub struct Tap {
    tap: Arc<async_io::Async<Iface>>,
}

impl Tap {
    pub fn new(name: &str, mode: tun_tap::Mode) -> Result<Block> {
        let tap = Iface::without_packet_info(name, mode)?;
        let tap = Arc::new(async_io::Async::new(tap)?);

        Ok(Block::new(
            BlockMetaBuilder::new("Tap").build(),
            StreamIoBuilder::new().build(),
            MessageIoBuilder::new()
                .add_input("in", Self::input_handler)
                .add_output("out")
                .build(),
            Tap { tap },
        ))
    }

    #[message_handler]
    async fn input_handler(
        &mut self,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
        p: Pmt,
    ) -> Result<Pmt> {
        if let Pmt::Blob(data) = p {
            match self.tap.write_with(|x| x.send(&data)).await {
                Ok(n) => assert_eq!(n, data.len()),
                Err(_) => {
                    eprintln!("Failed to write packet to TAP device, dropping.  Is the device up?")
                }
            }
        }

        Ok(Pmt::Null)
    }
}

#[async_trait]
impl Kernel for Tap {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        _sio: &mut StreamIo,
        mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        if io.block_on.is_some() {
            return Ok(());
        }

        let mut buf = vec![0; 1500];

        loop {
            match self.tap.as_ref().as_ref().recv(&mut buf) {
                Ok(len) => mio.post(0, Pmt::Blob(buf[..len].to_vec())).await,
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                _ => bail!("Error reading from tap interface"),
            }
        }

        let t2 = self.tap.clone();
        io.block_on(async move {
            t2.readable().await.unwrap();
        });

        Ok(())
    }
}
