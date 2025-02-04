use defmt::panic;

pub struct Accel {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl Accel {
    pub fn get_side(&self) -> Side {
        const TRESHOLD: i16 = 127;

        if self.z > TRESHOLD {
            Side::One
        } else if self.z < (-TRESHOLD) {
            Side::Two
        } else if self.y > TRESHOLD {
            Side::Three
        } else if self.y < (-TRESHOLD) {
            Side::Four
        } else if self.x > TRESHOLD {
            Side::Five
        } else if self.x < (-TRESHOLD) {
            Side::Six
        } else {
            panic!("Unknown Side values")
        }
    }
}

/// Defines the six sides the cube has.
#[derive(PartialEq, Clone, Copy)]
pub enum Side {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

// struct TTCConfig {
//     sides: u8,
// }

// impl TTCConfig {
//     pub fn gen_entry() {}
// }

pub struct Entry {
    pub side: u8,
    pub duration: u64,
}

impl Entry {
    pub fn new(side: Side, duration: u64) -> Self {
        Self {
            side: side as u8,
            duration,
        }
    }
}
