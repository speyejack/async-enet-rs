use std::fmt::Display;

use crate::error::ENetError;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct PeerID(pub u16);

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct OutgoingPeerID(pub u16);

impl From<OutgoingPeerID> for PeerID {
    fn from(value: OutgoingPeerID) -> Self {
        Self(value.0)
    }
}

impl From<PeerID> for OutgoingPeerID {
    fn from(value: PeerID) -> Self {
        Self(value.0)
    }
}

macro_rules! impl_convers_ids {
    ($ty: path) => {
        impl Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<u16> for $ty {
            fn from(value: u16) -> Self {
                $ty(value)
            }
        }

        impl From<u8> for $ty {
            fn from(value: u8) -> Self {
                let val: u16 = value.into();
                val.into()
            }
        }

        impl From<$ty> for u16 {
            fn from(value: $ty) -> Self {
                value.0
            }
        }

        impl TryFrom<$ty> for u8 {
            type Error = ENetError;

            fn try_from(value: $ty) -> std::result::Result<Self, Self::Error> {
                let val: u16 = value.0.into();
                Ok(val.try_into()?)
            }
        }
    };
}

impl_convers_ids!(PeerID);
impl_convers_ids!(OutgoingPeerID);
