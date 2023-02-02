use futuresdr::num_complex::Complex32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sym {
    A,
    B,
    C,
    D,
}

impl From<Sym> for u8 {
    fn from(value: Sym) -> Self {
        match value {
            Sym::A => 0b00,
            Sym::B => 0b01,
            Sym::C => 0b10,
            Sym::D => 0b11,
        }
    }
}

impl Sym {
    fn convert_nibble(nibble: u8) -> Sym {
        match nibble & 0x3 {
            0b00 => Sym::A,
            0b01 => Sym::B,
            0b10 => Sym::C,
            0b11 => Sym::D,
            _ => unreachable!(),
        }
    }

    pub fn syms_from_byte(byte: u8) -> Vec<Sym> {
        let mut ret = Vec::new();
        ret.push(Self::convert_nibble(byte >> 6));
        ret.push(Self::convert_nibble(byte >> 4));
        ret.push(Self::convert_nibble(byte >> 2));
        ret.push(Self::convert_nibble(byte));
        ret
    }

    pub fn inc(&self) -> Self {
        match self {
            Sym::A => Sym::C,
            Sym::B => Sym::A,
            Sym::C => Sym::D,
            Sym::D => Sym::B,
        }
    }

    pub fn dec(&self) -> Self {
        match self {
            Sym::A => Sym::B,
            Sym::B => Sym::D,
            Sym::C => Sym::A,
            Sym::D => Sym::C,
        }
    }

    pub fn add(&self, n: usize) -> Self {
        match n & 0x3 {
            0 => *self,
            1 => self.inc(),
            2 => self.inc().inc(),
            3 => self.inc().inc().inc(),
            _ => unreachable!(),
        }
    }

    pub fn sub(&self, n: usize) -> Self {
        match n & 0x3 {
            0 => *self,
            1 => self.dec(),
            2 => self.dec().dec(),
            3 => self.dec().dec().dec(),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sym;

    #[test]
    fn sym_rotation_symmetry() {
        let x = vec![Sym::A, Sym::B, Sym::C, Sym::D];

        assert_eq!(
            x,
            x.iter()
                .map(|x| x.inc().inc().inc().inc())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.dec()).collect::<Vec<_>>(),
            x.iter().map(|x| x.inc().inc().inc()).collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.dec().dec()).collect::<Vec<_>>(),
            x.iter().map(|x| x.inc().inc()).collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.dec().dec().dec()).collect::<Vec<_>>(),
            x.iter().map(|x| x.inc()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn sym_addition() {
        let x = vec![Sym::A, Sym::B, Sym::C, Sym::D];

        assert_eq!(x.iter().map(|x| x.add(4)).collect::<Vec<_>>(), x);
        assert_eq!(
            x.iter().map(|x| x.add(5)).collect::<Vec<_>>(),
            x.iter().map(|x| x.add(1)).collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.add(9)).collect::<Vec<_>>(),
            x.iter().map(|x| x.add(1)).collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.add(6)).collect::<Vec<_>>(),
            x.iter().map(|x| x.add(2)).collect::<Vec<_>>()
        );
        assert_eq!(
            x.iter().map(|x| x.add(7)).collect::<Vec<_>>(),
            x.iter().map(|x| x.add(3)).collect::<Vec<_>>()
        );
        assert_eq!(x.iter().map(|x| x.add(8)).collect::<Vec<_>>(), x);
    }
}

const N: f32 = 0.3;

impl From<&Sym> for Complex32 {
    fn from(value: &Sym) -> Self {
        let _pi = std::f32::consts::PI;

        match value {
            Sym::A => Complex32::new(N, N),
            Sym::B => Complex32::new(-N, N),
            Sym::C => Complex32::new(N, -N),
            Sym::D => Complex32::new(-N, -N),
        }
    }
}

pub type Symbol = Option<Sym>;
