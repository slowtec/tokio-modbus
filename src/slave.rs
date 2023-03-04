// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus devices

use std::{fmt, num::ParseIntError, str::FromStr};

/// Slave identifier
pub type SlaveId = u8;

/// A single byte for addressing Modbus slave devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slave(pub SlaveId);

impl Slave {
    /// The special address for sending a broadcast message to all
    /// connected Modbus slave devices at once. Broadcast messages
    /// are one-way and sent from the master to all slaves, i.e.
    /// a request without a response.
    ///
    /// Some devices may use a custom id from the reserved range
    /// 248-255 for broadcasting.
    #[must_use]
    pub const fn broadcast() -> Self {
        Slave(0)
    }

    /// The minimum address of a single Modbus slave device.
    #[must_use]
    pub const fn min_device() -> Self {
        Slave(1)
    }

    /// The maximum address of a single Modbus slave device.
    #[must_use]
    pub const fn max_device() -> Self {
        Slave(247)
    }

    /// The reserved address for sending a message to a directly
    /// connected Modbus TCP device, i.e. if not forwarded through
    /// a TCP/RTU gateway according to the unit identifier.
    ///
    /// [Modbus Messaging on TCP/IP Implementation Guide](http://www.modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf), page 23
    /// "On TCP/IP, the Modbus server is addressed using its IP address; therefore,
    /// the Modbus Unit Identifier is useless. The value 0xFF has to be used."
    #[must_use]
    pub const fn tcp_device() -> Self {
        Slave(255)
    }

    /// Check if the [`SlaveId`] is used for broadcasting
    #[must_use]
    pub fn is_broadcast(self) -> bool {
        self == Self::broadcast()
    }

    /// Check if the [`SlaveId`] addresses a single device
    #[must_use]
    pub fn is_single_device(self) -> bool {
        self >= Self::min_device() && self <= Self::max_device()
    }

    /// Check if the [`SlaveId`] is reserved
    #[must_use]
    pub fn is_reserved(self) -> bool {
        self > Self::max_device()
    }
}

impl From<SlaveId> for Slave {
    fn from(from: SlaveId) -> Self {
        Slave(from)
    }
}

impl From<Slave> for SlaveId {
    fn from(from: Slave) -> Self {
        from.0
    }
}

impl FromStr for Slave {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let slave_id = match s.parse::<u8>() {
            Ok(slave_id) => Ok(slave_id),
            Err(err) => {
                if let Some(stripped) = s.strip_prefix("0x") {
                    u8::from_str_radix(stripped, 16)
                } else {
                    Err(err)
                }
            }
        }?;
        Ok(Slave(slave_id))
    }
}

impl fmt::Display for Slave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:0>2X})", self.0, self.0)
    }
}

/// Stateful management of the currently active device.
///
/// RTU devices are addressed by their assigned *slave id*.
///
/// TCP devices are either addressed directly (= implicitly) by using the
/// reserved *unit id* `Slave::tcp_device() = 0xFF` (default) or indirectly
/// through an TCP/RTU gateway by setting the *unit id* to the desired
/// *slave id*.
///
/// The names *slave id* and *unit id* are used synonymously depending
/// on the context. This library consistently adopted the term *slave*.
pub trait SlaveContext {
    /// Select a slave device for all subsequent outgoing requests.
    fn set_slave(&mut self, slave: Slave);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dec() {
        assert_eq!(Slave(0), Slave::from_str("0").unwrap());
        assert_eq!(Slave(123), Slave::from_str("123").unwrap());
        assert_eq!(Slave(255), Slave::from_str("255").unwrap());
        assert!(Slave::from_str("-1").is_err());
        assert!(Slave::from_str("256").is_err());
    }

    #[test]
    fn parse_hex() {
        assert_eq!(Slave(0), Slave::from_str("0x00").unwrap());
        assert_eq!(Slave(123), Slave::from_str("0x7b").unwrap());
        assert_eq!(Slave(123), Slave::from_str("0x7B").unwrap());
        assert_eq!(Slave(255), Slave::from_str("0xff").unwrap());
        assert_eq!(Slave(255), Slave::from_str("0xFF").unwrap());
        assert!(Slave::from_str("0X00").is_err());
        assert!(Slave::from_str("0x100").is_err());
        assert!(Slave::from_str("0xfff").is_err());
        assert!(Slave::from_str("0xFFF").is_err());
    }

    #[test]
    fn format() {
        assert!(format!("{}", Slave(123)).contains("123"));
        assert!(format!("{}", Slave(0x7B)).contains("0x7B"));
        assert!(!format!("{}", Slave(0x7B)).contains("0x7b"));
    }
}
