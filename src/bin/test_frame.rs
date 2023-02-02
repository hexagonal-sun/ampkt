use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use futuresdr::{
    blocks::{MessageSource, SoapySinkBuilder},
    macros::connect,
    runtime::{Flowgraph, Runtime},
};

use ampkt::{frame::FrameEncoder, qam::QamMod};

#[derive(Parser)]
struct Args {
    soapy_device: String,
    tx_freq: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut fg = Flowgraph::new();

    let test_frame = MessageSource::new(
        futuresdr::runtime::Pmt::Blob([0xde, 0xad, 0xbe, 0xef].to_vec()),
        Duration::from_secs(0),
        None,
    );

    let frame_encoder = FrameEncoder::new();

    let qam_mod = QamMod::new(10);

    let dev =
        soapysdr::Device::new(args.soapy_device.as_str()).context("Could not find SDR device")?;

    let soapy_sink = SoapySinkBuilder::new()
        .device(dev)
        .sample_rate(800_000.0)
        .freq(args.tx_freq)
        .build();

    connect!(fg, test_frame | frame_encoder > qam_mod > soapy_sink);

    Runtime::new().run(fg)?;

    Ok(())
}
