use std::time::Duration;

use anyhow::Result;
use futuresdr::{
    blocks::{FileSink, MessageSource, Throttle},
    macros::connect,
    num_complex::Complex32,
    runtime::{Flowgraph, Runtime},
};

use crate::qam::Qam;

mod qam;

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let msg_src = MessageSource::new(
        futuresdr::runtime::Pmt::Blob(vec![0xde, 0xad, 0xbe, 0xef]),
        Duration::from_secs(0),
        Some(1),
    );

    let qam = Qam::new();

    let throttle = Throttle::<Complex32>::new(1_000.0);

    let file_sink = FileSink::<Complex32>::new("out.cf32");

    connect!(fg, msg_src | qam > throttle > file_sink);

    Runtime::new().run(fg)?;

    Ok(())
}
