use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceId(pub u8);

impl DeviceId {
    /// The Modbus address for sending a broadcast message to all
    /// connected slave devices.
    pub const fn broadcast() -> Self {
        DeviceId(0)
    }

    /// The minimum slave address of a Modbus device.
    pub const fn min_slave() -> Self {
        DeviceId(1)
    }

    /// The maximum slave address of a Modbus device.
    pub const fn max_slave() -> Self {
        DeviceId(247)
    }

    pub fn is_broadcast(self) -> bool {
        self == Self::broadcast()
    }

    pub fn is_slave(self) -> bool {
        self >= Self::min_slave() && self <= Self::max_slave()
    }

    pub fn is_reserved(self) -> bool {
        self > Self::max_slave()
    }
}

impl From<u8> for DeviceId {
    fn from(from: u8) -> Self {
        DeviceId(from)
    }
}

impl From<DeviceId> for u8 {
    fn from(from: DeviceId) -> Self {
        from.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:0>2X}", self.0)
    }
}
