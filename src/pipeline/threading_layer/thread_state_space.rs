use num_enum::TryFromPrimitive;
use std::string::ToString;


#[repr(u8)]
#[derive(PartialEq, Debug, TryFromPrimitive, strum::Display, Clone)]
pub enum ThreadStateSpace {
    RUNNING = 0,
    PAUSED = 1,
    KILLED = 2
}
impl ThreadStateSpace {
    pub fn to_u8_ref(&self) -> u8 {
        match self {
            Self::RUNNING => 0,
            Self::PAUSED => 1,
            Self::KILLED => 2,
        }
    }
}