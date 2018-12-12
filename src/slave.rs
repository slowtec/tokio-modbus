use std::fmt;

pub type SlaveId = u8;

/// A single byte for addressing Modbus slave devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slave(pub SlaveId);

impl Slave {
    /// The special address for sending a broadcast message to all
    /// connected Modbus slave devices at once. Broadcast messages
    /// are one-way and sent from the master to all slaves, i.e.
    /// a request without a response.
    pub const fn broadcast() -> Self {
        Slave(0)
    }

    /// The minimum address of a single Modbus slave device.
    pub const fn min_device() -> Self {
        Slave(1)
    }

    /// The maximum address of a single Modbus slave device.
    pub const fn max_device() -> Self {
        Slave(247)
    }

    /// The reserved address for sending a message to a directly
    /// connected Modbus TCP device, i.e. if not forwarded through
    /// a TCP/RTU gateway according to the unit identifier.
    ///
    /// [MODBUS Messaging on TCP/IP Implementation Guide](http://www.modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf), page 23
    /// "On TCP/IP, the MODBUS server is addressed using its IP address; therefore,
    /// the MODBUS Unit Identifier is useless. The value 0xFF has to be used."
    pub const fn tcp_device() -> Self {
        Slave(255)
    }

    pub fn is_broadcast(self) -> bool {
        self == Self::broadcast()
    }

    pub fn is_single_device(self) -> bool {
        self >= Self::min_device() && self <= Self::max_device()
    }

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

impl fmt::Display for Slave {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:0>2X}", self.0)
    }
}

pub trait SlaveContext {
    /// Select a slave device for all subsequent outgoing requests.
    fn set_slave(&mut self, slave: Slave);
}
