//! Host-side I2C bus simulation helpers.

use hal_api::error::I2cError;
use hal_api::i2c::I2cBus;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

pub trait VirtualI2cDevice {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError>;

    fn read(&mut self, _buffer: &mut [u8]) -> Result<(), I2cError> {
        Err(I2cError::BusError)
    }

    fn write_read(&mut self, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError> {
        self.write(bytes)?;
        self.read(buffer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualI2cOperation {
    Write {
        addr: u8,
        bytes: Vec<u8>,
    },
    Read {
        addr: u8,
        len: usize,
    },
    WriteRead {
        addr: u8,
        bytes: Vec<u8>,
        len: usize,
    },
}

type SharedVirtualDevice = Rc<RefCell<Box<dyn VirtualI2cDevice>>>;

#[derive(Default)]
struct VirtualI2cBusState {
    devices: Vec<(u8, SharedVirtualDevice)>,
    operations: Vec<VirtualI2cOperation>,
}

#[derive(Clone, Default)]
pub struct VirtualI2cBus {
    state: Rc<RefCell<VirtualI2cBusState>>,
}

impl VirtualI2cBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach_device<D>(&self, addr: u8, device: D)
    where
        D: VirtualI2cDevice + 'static,
    {
        let mut state = self.state.borrow_mut();
        state.devices.retain(|(candidate, _)| *candidate != addr);
        state
            .devices
            .push((addr, Rc::new(RefCell::new(Box::new(device)))));
    }

    pub fn operations(&self) -> Vec<VirtualI2cOperation> {
        self.state.borrow().operations.clone()
    }

    pub fn operation_count(&self) -> usize {
        self.state.borrow().operations.len()
    }

    pub fn attached_addresses(&self) -> Vec<u8> {
        let mut addresses = self
            .state
            .borrow()
            .devices
            .iter()
            .map(|(addr, _)| *addr)
            .collect::<Vec<_>>();
        addresses.sort_unstable();
        addresses
    }

    fn with_device<T>(
        &self,
        addr: u8,
        operation: impl FnOnce(&mut dyn VirtualI2cDevice) -> Result<T, I2cError>,
    ) -> Result<T, I2cError> {
        let device = self
            .state
            .borrow()
            .devices
            .iter()
            .find(|(candidate, _)| *candidate == addr)
            .map(|(_, device)| Rc::clone(device))
            .ok_or(I2cError::InvalidAddress)?;
        let mut device = device.borrow_mut();
        operation(device.as_mut())
    }
}

fn push_operation(state: &mut VirtualI2cBusState, operation: VirtualI2cOperation) {
    const MAX_RECORDED_OPERATIONS: usize = 256;

    if state.operations.len() >= MAX_RECORDED_OPERATIONS {
        state.operations.remove(0);
    }
    state.operations.push(operation);
}

impl I2cBus for VirtualI2cBus {
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        push_operation(
            &mut self.state.borrow_mut(),
            VirtualI2cOperation::Write {
                addr,
                bytes: bytes.to_vec(),
            },
        );
        self.with_device(addr, |device| device.write(bytes))
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        push_operation(
            &mut self.state.borrow_mut(),
            VirtualI2cOperation::Read {
                addr,
                len: buffer.len(),
            },
        );
        self.with_device(addr, |device| device.read(buffer))
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        push_operation(
            &mut self.state.borrow_mut(),
            VirtualI2cOperation::WriteRead {
                addr,
                bytes: bytes.to_vec(),
                len: buffer.len(),
            },
        );
        self.with_device(addr, |device| device.write_read(bytes, buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestDevice {
        writes: Vec<Vec<u8>>,
        next_read: Vec<u8>,
    }

    impl VirtualI2cDevice for TestDevice {
        fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
            self.writes.push(bytes.to_vec());
            Ok(())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<(), I2cError> {
            if buffer.len() != self.next_read.len() {
                return Err(I2cError::BusError);
            }
            buffer.copy_from_slice(&self.next_read);
            Ok(())
        }

        fn write_read(&mut self, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError> {
            self.write(bytes)?;
            self.read(buffer)
        }
    }

    #[test]
    fn virtual_i2c_bus_routes_operations_to_attached_devices() {
        let bus = VirtualI2cBus::new();
        bus.attach_device(
            0x77,
            TestDevice {
                writes: Vec::new(),
                next_read: vec![0x60],
            },
        );
        let mut bus_handle = bus.clone();
        let mut chip_id = [0u8; 1];

        bus_handle.write_read(0x77, &[0xD0], &mut chip_id).unwrap();

        assert_eq!(chip_id, [0x60]);
        assert_eq!(
            bus.operations(),
            vec![VirtualI2cOperation::WriteRead {
                addr: 0x77,
                bytes: vec![0xD0],
                len: 1,
            }]
        );
    }

    #[test]
    fn virtual_i2c_bus_rejects_unknown_address() {
        let mut bus = VirtualI2cBus::new();
        let mut buffer = [0u8; 1];

        assert_eq!(bus.read(0x42, &mut buffer), Err(I2cError::InvalidAddress));
    }
}
