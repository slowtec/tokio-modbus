#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

/// The Modbus address for sending a broadcast message to
/// all connected stations (RTU) or slave devices (TCP).
/// TODO: Sending broadcast messages is not supported yet!
#[allow(dead_code)]
pub(crate) const BROADCAST_ADDRESS: u8 = 0x00;

/// The minimum Modbus address of a connected station (RTU) or slave device (TCP).
pub(crate) const MIN_ADDRESS: u8 = 1;

/// The maximum Modbus address of a connected station (RTU) or slave device (TCP).
pub(crate) const MAX_ADDRESS: u8 = 247;
