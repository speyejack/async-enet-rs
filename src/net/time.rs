use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PacketTime(u16);

impl From<u16> for PacketTime {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<PacketTime> for u16 {
    fn from(value: PacketTime) -> Self {
        value.0
    }
}

impl PacketTime {
    pub fn from_duration(dur: &Duration) -> Self {
        let time = (dur.as_millis() & 0xFFFF) as u16;
        Self(time)
    }

    pub fn to_duration(&self, curr: &Duration) -> Option<Duration> {
        let lower: u16 = self.0.into();
        let lower = lower as u64;
        let curr_mill = curr.as_millis() as u64;
        let mut dur: u64 = curr_mill & !((!0_u16) as u64) | lower;
        if (dur & 0x8000) > (curr_mill & 0x8000) {
            dur -= 0x10000;
        }
        let dur = Duration::from_millis(dur);
        if &dur > curr {
            return None;
        }

        Some(dur)
    }
}
