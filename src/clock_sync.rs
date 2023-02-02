use anyhow::Result;
use futuresdr::{
    async_trait::async_trait,
    num_complex::Complex32,
    runtime::{
        Block, BlockMeta, BlockMetaBuilder, Kernel, MessageIo, MessageIoBuilder, StreamIo,
        StreamIoBuilder, WorkIo,
    },
};

pub struct ClockSync {
    skip: i32,
    n: i32,
    err_gain: f32,
    window: Vec<Complex32>,
}

impl ClockSync {
    pub fn new(sps: u32, err_gain: f32) -> Block {
        assert!(sps > 3);

        Block::new(
            BlockMetaBuilder::new("ClockSync").build(),
            StreamIoBuilder::new()
                .add_input("in", std::mem::size_of::<Complex32>())
                .add_output("out", std::mem::size_of::<Complex32>())
                .build(),
            MessageIoBuilder::new().build(),
            ClockSync {
                skip: sps as i32,
                n: sps as i32,
                err_gain,
                window: Vec::with_capacity(3),
            },
        )
    }

    fn calc_error(&mut self) -> f32 {
        let max_delta = self.skip as f32 / 2.0;

        let x0 = self.window[0].re;
        let x1 = self.window[1].re;
        let x2 = self.window[2].re;

        self.window.clear();

        let mut err = ((x2 - x0) * x1) * self.err_gain;

        if err < -max_delta {
            err = -max_delta
        } else if err > max_delta {
            err = max_delta;
        }

        err
    }

    fn push_samp(&mut self, s: Complex32) -> Option<Complex32> {
        self.n -= 1;

        match self.n {
            2 => {
                self.window.push(s);
                None
            }
            1 => {
                self.window.push(s);
                Some(s)
            }
            0 => {
                self.window.push(s);

                self.n = self.skip + self.calc_error().round() as i32;

                None
            }
            _ => None,
        }
    }
}

#[async_trait]
impl Kernel for ClockSync {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        let input = sio.input(0);
        let is = input.slice::<Complex32>();
        let out_output = sio.output(0);
        let os = out_output.slice::<Complex32>();
        let mut consumed = 0;
        let mut produced = 0;

        if sio.input(0).finished() {
            io.finished = true;
        }

        let mut in_iter = is.iter();

        for out_samp in os.iter_mut() {
            'inner: for in_samp in &mut in_iter {
                consumed += 1;
                if let Some(o) = self.push_samp(*in_samp) {
                    *out_samp = o;
                    produced += 1;
                    break 'inner;
                }
            }
        }

        sio.input(0).consume(consumed);
        sio.output(0).produce(produced);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futuresdr::num_complex::Complex32;

    use super::ClockSync;

    #[test]
    fn err_convergence() {
        let mut cs = ClockSync {
            skip: 10,
            err_gain: 20.0,
            n: 0,
            window: Vec::with_capacity(3),
        };

        // samples are heading towards the symbol above 0. Err should be +ve so
        // we skip more samples.
        cs.window.push(Complex32::new(0.1, 0.0));
        cs.window.push(Complex32::new(0.2, 0.0));
        cs.window.push(Complex32::new(0.3, 0.0));

        assert!(cs.calc_error() > 0.0);

        // samples are heading away the symbol above 0. Err should be -ve so we
        // skip less samples.
        cs.window.push(Complex32::new(0.3, 0.0));
        cs.window.push(Complex32::new(0.2, 0.0));
        cs.window.push(Complex32::new(0.1, 0.0));

        assert!(cs.calc_error() < 0.0);

        // samples are heading towards the symbol below 0. Err should be +ve so
        // we skip more samples.
        cs.window.push(Complex32::new(-0.1, 0.0));
        cs.window.push(Complex32::new(-0.2, 0.0));
        cs.window.push(Complex32::new(-0.3, 0.0));

        assert!(cs.calc_error() > 0.0);

        // samples are heading towards the symbol below 0. Err should be -ve so
        // we skip less samples.
        cs.window.push(Complex32::new(-0.5, 0.0));
        cs.window.push(Complex32::new(-0.3, 0.0));
        cs.window.push(Complex32::new(-0.2, 0.0));

        assert!(cs.calc_error() < 0.0);

        // samples are at the symbol peak above 0. Err should be 0 so we don't
        // induce any more or less skip.
        cs.window.push(Complex32::new(0.3, 0.0));
        cs.window.push(Complex32::new(0.4, 0.0));
        cs.window.push(Complex32::new(0.3, 0.0));

        assert_eq!(cs.calc_error(), 0.0);

        // samples are at the symbol peak below 0. Err should be 0 so we don't induce
        // any more or less skip.
        cs.window.push(Complex32::new(-0.3, 0.0));
        cs.window.push(Complex32::new(-0.4, 0.0));
        cs.window.push(Complex32::new(-0.3, 0.0));

        assert_eq!(cs.calc_error(), 0.0);
    }
}
