use crate::sym::Sym;
use std::collections::BTreeMap;

pub const SYNC: [Sym; 16] = [
    Sym::A,
    Sym::B,
    Sym::A,
    Sym::C,
    Sym::A,
    Sym::D,
    Sym::B,
    Sym::D,
    Sym::A,
    Sym::C,
    Sym::B,
    Sym::B,
    Sym::A,
    Sym::B,
    Sym::D,
    Sym::C,
];

pub struct SymSync {
    rotations: BTreeMap<u32, usize>,
    n: u32,
}

impl SymSync {
    pub fn new() -> Self {
        let mut rotations = BTreeMap::new();

        (0..4).for_each(|n| {
            let mut sync_pattern: u32 = 0;

            SYNC.iter().for_each(|s| {
                Self::push_nibble(&mut sync_pattern, s.add(n));
            });

            rotations.insert(sync_pattern, n);
        });

        Self { rotations, n: 0 }
    }

    fn push_nibble(n: &mut u32, s: Sym) {
        *n <<= 2;
        *n |= u8::from(s) as u32;
    }

    pub fn push_sym(&mut self, s: Sym) -> Option<usize> {
        Self::push_nibble(&mut self.n, s);

        self.rotations.get(&self.n).copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::sym::Sym;

    use super::{SymSync, SYNC};

    fn run(mut s: SymSync, mut sym_transform: impl FnMut(Sym) -> Sym) -> Option<usize> {
        let mut it = SYNC.iter().peekable();

        while let Some(sym) = it.next() {
            let v = s.push_sym(sym_transform(*sym));

            if it.peek().is_none() {
                return v;
            } else {
                assert!(v.is_none())
            }
        }

        unreachable!();
    }

    #[test]
    fn sync_no_rot() {
        assert!(matches!(run(SymSync::new(), |s| s), Some(0)));
    }

    #[test]
    fn sync_rot_1() {
        assert!(matches!(run(SymSync::new(), |s| s.add(1)), Some(1)));
    }

    #[test]
    fn sync_rot_2() {
        assert!(matches!(run(SymSync::new(), |s| s.add(2)), Some(2)));
    }

    #[test]
    fn sync_rot_3() {
        assert!(matches!(run(SymSync::new(), |s| s.add(3)), Some(3)));
    }

    #[test]
    fn with_preamble() {
        let mut s = SymSync::new();

        s.push_sym(Sym::A);
        s.push_sym(Sym::B);
        s.push_sym(Sym::D);
        s.push_sym(Sym::A);

        assert!(matches!(run(s, |s| s.add(3)), Some(3)));
    }
}
