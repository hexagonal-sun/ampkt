use anyhow::{Context, Result};
use clap::Parser;
use futuresdr::{
    blocks::{SoapySinkBuilder, SoapySourceBuilder},
    macros::connect,
    runtime::{Flowgraph, Runtime},
};

use ampkt::{
    carrier_sync::CarrierSync,
    clock_sync::ClockSync,
    frame::{FrameDecoder, FrameEncoder},
    qam::{QamDemod, QamMod},
    tap::Tap,
};

#[derive(Parser)]
struct Args {
    #[clap(short,long)]
    tx_gain: f64,
    #[clap(short,long)]
    rx_gain: f64,
    soapy_device: String,
    tx_freq: f64,
    rx_freq: f64,
}

const SAMP_RATE: f64 = 800_000.0;

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let args = Args::parse();

    let tap = Tap::new("tap%d", tun_tap::Mode::Tap)?;

    // TX Blocks.
    let frame_encoder = FrameEncoder::new();

    let qam_mod = QamMod::new(10);

    let tx_dev =
        soapysdr::Device::new(args.soapy_device.as_str()).context("Could not find SDR device")?;
    let tx_soapy_dev = SoapySinkBuilder::new()
        .device(tx_dev)
        .sample_rate(SAMP_RATE)
        .freq(args.tx_freq)
        .gain(args.tx_gain)
        .build();

    // RX Blocks.
    let rx_dev =
        soapysdr::Device::new(args.soapy_device.as_str()).context("Could not find SDR device")?;

    let rx_soapy_dev = SoapySourceBuilder::new()
        .device(rx_dev)
        .sample_rate(SAMP_RATE)
        .freq(args.rx_freq)
        .gain(args.rx_gain)
        .build();

    let clock_sync = ClockSync::new(10, 20.0);

    let carrier_sync = CarrierSync::new();

    let qam_demod = QamDemod::new();

    let frame_decoder = FrameDecoder::new();

    connect!(fg,
             // TX Path
             tap | frame_encoder > qam_mod > tx_soapy_dev;
             // RX Path
             rx_soapy_dev > clock_sync > carrier_sync > qam_demod > frame_decoder | tap);

    Runtime::new().run(fg)?;

    Ok(())
}
