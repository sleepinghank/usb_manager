// Usb Manager

//! ### For example
//! ```rust
//!     let mut adapter = adapter::Adapter::new();
//!     adapter.start().unwrap();
//!     
//!     let devices = adapter.peripherals().unwrap();
//!     println!("len:{}", devices.len());
//!     let read = adapter.events().unwrap();
//!     loop {
//!         match read.recv() {
//!             Ok(v) => {
//!                 match v {
//!                     CentralEvent::DeviceAdd(id) => {
//!                         println!("Add:{:?}",id);
//!                     },
//!                     CentralEvent::DeviceRemove(device) => {
//!                         println!("Remove:{:?}",device.id);
//!                     },
//!                 }
//!             },
//!             Err(err) => println!("Err:{:?}",err),
//!         }
//!     }
//!```


mod device_interface;

mod pnp_detect;
mod manager;
mod utils;
pub mod adapter;
pub mod hid_device;


use thiserror::Error;
use std::result;
use windows::Win32::Foundation::GetLastError;
use uuid::Uuid;
use hid_device::HidDevice;

#[derive(Debug,Error)]
pub enum Error {
    #[error("Win32 error 0x{0:0X}")]
    Win32(u32),

    #[error("Device not turned on")]
    NotOpen,

    #[error("Device opening error")]
    OpenError,

    #[error("Device not found")]
    NotFound,

    #[error("Data exceeds the maximum length")]
    DataOverlength,
    
    #[error("{}", _0)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn win32() -> Self {
        unsafe { Self::Win32(GetLastError().0) }
    }
}


pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum CentralEvent {
    DeviceAdd(Uuid),
    DeviceRemove(HidDevice),
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {
        
        let mut adapter = adapter::Adapter::new();
        adapter.start().unwrap();

        let devices = adapter.peripherals().unwrap();
        println!("len:{}", devices.len());
        let read = adapter.events().unwrap();
        loop {
            match read.recv() {
                Ok(v) => {
                    match v {
                        CentralEvent::DeviceAdd(id) => {
                            println!("Add:{:?}",id);
                        },
                        CentralEvent::DeviceRemove(device) => {
                            println!("Remove:{:?}",device.id);
                        },
                    }
                },
                Err(err) => println!("Err:{:?}",err),
            }
        }
    }
}
