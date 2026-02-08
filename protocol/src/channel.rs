//! QUIC transport channel definitions.

/// Logical transport primitive used by a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    BidiStream,
    UniStream,
    Datagram,
}

/// Message delivery guarantee expected for a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    ReliableOrdered,
    ReliableUnordered,
    Unreliable,
}

/// Stable channel identifiers used by the protocol on top of QUIC.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuicChannel {
    /// Authentication, keepalive, world routing, acks.
    Control = 0,
    /// Chat and social signals.
    Chat = 1,
    /// Movement and high-frequency input updates.
    GameplayInput = 2,
    /// Critical gameplay events that must be reliable.
    GameplayEvent = 3,
    /// Economy/inventory/trade/cash operations.
    Economy = 4,
}

impl QuicChannel {
    /// Returns the required transport primitive for this channel.
    #[must_use]
    pub const fn transport(self) -> TransportKind {
        match self {
            Self::Control | Self::Chat | Self::GameplayEvent | Self::Economy => {
                TransportKind::BidiStream
            }
            Self::GameplayInput => TransportKind::Datagram,
        }
    }

    /// Returns the delivery guarantee for this channel.
    #[must_use]
    pub const fn delivery(self) -> DeliveryGuarantee {
        match self {
            Self::Control | Self::Chat | Self::GameplayEvent | Self::Economy => {
                DeliveryGuarantee::ReliableOrdered
            }
            Self::GameplayInput => DeliveryGuarantee::Unreliable,
        }
    }

    /// Indicates whether this channel carries operations that cannot be dropped.
    #[must_use]
    pub const fn is_critical(self) -> bool {
        matches!(self, Self::Economy | Self::GameplayEvent | Self::Control)
    }
}

impl core::convert::TryFrom<u8> for QuicChannel {
    type Error = InvalidChannel;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Control),
            1 => Ok(Self::Chat),
            2 => Ok(Self::GameplayInput),
            3 => Ok(Self::GameplayEvent),
            4 => Ok(Self::Economy),
            _ => Err(InvalidChannel(value)),
        }
    }
}

/// Error returned when an unknown channel id is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidChannel(pub u8);

impl core::fmt::Display for InvalidChannel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid QUIC channel id {}", self.0)
    }
}

impl std::error::Error for InvalidChannel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_input_is_datagram_unreliable() {
        assert_eq!(
            QuicChannel::GameplayInput.transport(),
            TransportKind::Datagram
        );
        assert_eq!(
            QuicChannel::GameplayInput.delivery(),
            DeliveryGuarantee::Unreliable
        );
    }

    #[test]
    fn economy_is_critical() {
        assert!(QuicChannel::Economy.is_critical());
        assert!(!QuicChannel::GameplayInput.is_critical());
    }
}
