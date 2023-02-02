use std::f32::consts::PI;

use futuresdr::{blocks::Apply, num_complex::Complex32, runtime::Block};

pub struct CarrierSync;

impl CarrierSync {
    fn threshold(x: f32) -> f32 {
        if x > 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    fn calc_error(s: Complex32) -> f32 {
        (Self::threshold(s.im) * s.re) - (Self::threshold(s.re) * s.im)
    }

    pub fn new() -> Block {
        let mut phase_comp: f32 = 0.0;

        Apply::new(move |x: &Complex32| {
            let (mag, mut phase) = x.to_polar();

            phase += phase_comp;

            let ret = Complex32::from_polar(mag, phase);
            phase_comp += Self::calc_error(ret);

            while phase_comp > (2.0 * PI) {
                phase_comp -= 2.0 * PI;
            }

            while phase_comp < (-2.0 * PI) {
                phase_comp += 2.0 * PI;
            }

            ret
        })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use futuresdr::num_complex::Complex32;

    use super::CarrierSync;

    #[test]
    fn error_fn() -> Result<()> {
        assert_eq!(CarrierSync::calc_error(Complex32::new(1.0, 1.0)), 0.0);
        assert_eq!(CarrierSync::calc_error(Complex32::new(-1.0, 1.0)), 0.0);
        assert_eq!(CarrierSync::calc_error(Complex32::new(1.0, -1.0)), 0.0);
        assert_eq!(CarrierSync::calc_error(Complex32::new(-1.0, -1.0)), 0.0);

        assert!(CarrierSync::calc_error(Complex32::new(1.0, 1.1)) < 0.0);
        assert!(CarrierSync::calc_error(Complex32::new(1.0, 0.9)) > 0.0);

        assert_eq!(
            CarrierSync::calc_error(Complex32::new(1.0, 1.1)).abs(),
            CarrierSync::calc_error(Complex32::new(1.0, 0.9)).abs()
        );
        Ok(())
    }
}
