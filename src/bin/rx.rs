use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use futuresdr::{
    blocks::{FileSink, FileSource, MessagePipe, SoapySourceBuilder},
    futures::{channel::mpsc, executor::block_on, StreamExt},
    macros::connect,
    num_complex::Complex32,
    runtime::{Flowgraph, Pmt, Runtime},
};

use ampkt::{carrier_sync::CarrierSync, clock_sync::ClockSync, frame::FrameDecoder, qam::QamDemod};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: RxType,
}

#[derive(clap::Subcommand)]
enum RxType {
    File {
        path: PathBuf,
    },
    SDR {
        soapy_device: String,
        rx_freq: f64,
        #[clap(short, long)]
        gain: f64,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut fg = Flowgraph::new();

    let src = match args.cmd {
        RxType::File { path } => FileSource::<Complex32>::new(path.to_string_lossy(), false),
        RxType::SDR {
            soapy_device,
            rx_freq,
            gain,
        } => {
            let dev = soapysdr::Device::new(soapy_device.as_str())
                .context("Could not find SDR device")?;
            SoapySourceBuilder::new()
                .device(dev)
                .sample_rate(800_000.0)
                .freq(rx_freq)
                .gain(gain)
                .build()
        }
    };

    let clock_sync = ClockSync::new(10, 20.0);

    let carrier_sync = CarrierSync::new();

    let qam_demod = QamDemod::new();

    let frame_decoder = FrameDecoder::new();

    let (tx, mut rx) = mpsc::channel::<Pmt>(100);

    let message_sink = MessagePipe::new(tx);

    let raw_signal_sink = FileSink::<Complex32>::new("raw.cf32");
    let clock_sync_sink = FileSink::<Complex32>::new("clock_sync.cf32");
    let carrier_sync_sink = FileSink::<Complex32>::new("carrier_sync.cf32");

    connect!(fg, src > clock_sync > carrier_sync > qam_demod > frame_decoder | message_sink;
             src > raw_signal_sink;
             carrier_sync > carrier_sync_sink;
             clock_sync > clock_sync_sink);

    let rt = Runtime::new();
    let (_fg, _handle) = block_on(rt.start(fg));

    rt.block_on(async move {
        while let Some(x) = rx.next().await {
            match x {
                Pmt::Blob(frame) => println!("RX'd frame: {frame:X?}"),
                Pmt::Null => break,
                _ => eprintln!("Unexpected message type from qam demot"),
            }
        }
    });

    Ok(())
}
