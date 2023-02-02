use anyhow::{Context, Result};
use futuresdr::{
    blocks::{FileSink, SoapySourceBuilder},
    macros::connect,
    num_complex::Complex32,
    runtime::{Flowgraph, Runtime},
};

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let dev = soapysdr::Device::new("driver=bladerf").context("Could not find SDR device")?;

    let soapy_src = SoapySourceBuilder::new()
        .device(dev)
        .sample_rate(800_000.0)
        .freq(433_000_000.0)
        .gain(60.0)
        .build();

    let file_sink = FileSink::<Complex32>::new("out.cf32");

    connect!(fg, soapy_src > file_sink);

    Runtime::new().run(fg)?;

    Ok(())
}
