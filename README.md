# AMPKT - Amateur radio packet explorer

This is a project to facilitate experimentation with connecting two Linux
machine's network stacks via the amateur radio bands.

## Required Hardware

You will need two full-duplex SDRs, one for each machine. They will also need to
be compatible with SoapySDR. 

Hardware that is known to work:
 - USRP N210
 - BladeRF
 - LimeSDR
 

## Getting Started
 
 1. Build the project:
 
 ```console
 cargo build --release
 ```
 
 2. Connect your SDR hardware and start the `ampkt` binary (note that you will
    probably need to run with `sudo` so that the `tap` interface can be
    created):
 
 ```console
 sudo ./target/release/ampkt "driver=bladerf" 433000000 435000000
 ```
 
 This will start ampkt using bladerf hardware, Txing on 433MHz and Rxing on
 435MHz.
 
 3. On a second machine do the same, ensuring that you swap the frequencies.
 
 4. On both machines, bring up the tap interface and assign an IP address:
 
 ```console
 sudo ifconfig tap0 up
 sudo ip add 10.0.0.1/24 dev tap0
 ```
     
 Making sure you use a different IP address on the other machine.
 
 5. At this point you should have working comms. You can check running `ping`and
    seeing whether you get a reply from the other machine
    
```console
ping 10.0.0.2
```
 

## Technical Information

A Linux [TAP](https://www.kernel.org/doc/html/v5.8/networking/tuntap.html)
interface is created. This interface is used as the sink for the RX path and the
source for the TX path.

### Tx Path

Each incoming packet that is read from the tap interface is converted into a
frame. The frame consists of a sync header (repeated twice) a frame size (as
u16) followed by the packet data. We then convert the frame to a stream of
symbols. We use QPSK modulation which means we have two bits per symbol. Each
2-bit nibble is converted into a symbol using the following map:

```
A => 0b00
B => 0b01
C => 0b10
D => 0b11
```

We then module the symbol stream into QPSK with the following constellation:

```
       Im
       |
    B  |   A
       |
       |
-------------> re
       |
       |
    D  |   C
       |
```

The output of the QPSK modulator is then sent to the SDR for Tx.

### Rx Path

Samples from the SDR are first sent into the clock sync block. This block
decimates the incoming stream by selection of particular samples from the input
stream. The number of samples that are 'skipped' during selection is shifted by
an error function which attempts to pick the sample at the peak of a symbol.

The decimated stream is then sent into the carrier sync block. This attempts to
compensate for any difference in clocks between the SDRs by 'de-reotating' the
constellation. When this block has 'locked' the output should be stable samples
in each quadrent of the constellation plot.

Next, the samples passed through the QPSK demodulator which converts the samples
into a stream of symbols.

The symbol stream is then passed through a frame decoder. This block attempts to
create the original packet of data from a stream of symbols. We use a SYNC
header of 16-bytes (repeated twice) to resolve the phase ambiguity. Then the
computed difference in phase is applied to all incoming symbols to decode the
frame.

Finally the frames are written to the TAP interface for injection into the Linux
kernel network stack.
