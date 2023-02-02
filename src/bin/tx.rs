use anyhow::{Context, Result};
use futuresdr::{
    blocks::SoapySinkBuilder,
    macros::connect,
    runtime::{Flowgraph, Runtime},
};

use ampkt::{frame::FrameEncoder, qam::QamMod, tap::Tap};

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let tap = Tap::new("tap%d", tun_tap::Mode::Tap).unwrap();

    let frame_encoder = FrameEncoder::new();

    let qam_mod = QamMod::new(10);

    let dev = soapysdr::Device::new("driver=uhd,addr=192.168.50.2")
        .context("Could not find SDR device")?;

    let soapy_sink = SoapySinkBuilder::new()
        .device(dev)
        .sample_rate(800_000.0)
        .freq(433_000_000.0)
        .gain(0.0)
        .build();

    connect!(fg, tap | frame_encoder > qam_mod > soapy_sink);

    Runtime::new().run(fg)?;

    Ok(())
}
