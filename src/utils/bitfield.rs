use std::ops;

pub struct Bitfield {
    bits: Vec<bool>,
}

impl Bitfield {
    pub fn newt(size: usize) -> Bitfield {
        return Bitfield {
            bits: vec![false; size],
        };
    }

    pub fn get(&self, index: usize) -> bool {
        return self.bits[index];
    }

    pub fn set(&mut self, index: usize, val: bool) {
        self.bits[index] = val;
    }
}

impl ops::BitOr<Bitfield> for Bitfield {
    type Output = Bitfield;

    fn bitor(self, rhs: Bitfield) -> Self::Output {
        Bitfield {
            bits: self
                .bits
                .iter()
                .zip(rhs.bits.iter())
                .map(|(x, y)| x | y)
                .collect(),
        }
    }
}

impl ops::BitOrAssign<Bitfield> for Bitfield {
    fn bitor_assign(&mut self, rhs: Bitfield) {
        self.bits = self
            .bits
            .iter()
            .zip(rhs.bits.iter())
            .map(|(x, y)| x | y)
            .collect();
    }
}
