use std::{sync::{ Arc, Mutex}, thread::{JoinHandle,spawn}};
use std::fmt::{self, Debug, Formatter};
use anyhow::{Result, Ok};
use crossbeam_channel::Receiver;
use uuid::Uuid;

use super::{
    Error,
    CentralEvent,
    manager::Manager,
    hid_device::{HidDevice,all_hid_device},
    pnp_detect::PnPDetectWindows,
};

#[derive(Clone)]
pub struct Adapter {
    manager: Arc<Manager>,
    thread_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Debug for Adapter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Adapter")
            .field("manager", &self.manager)
            .finish()
    }
}


impl Adapter {
    pub fn new() -> Self {
        let manager = Arc::new(Manager::new());
        Self {  
            manager, 
            thread_handle:Arc::new(Mutex::new(None)), 
        }
    }

    pub fn start(&self) -> Result<()> {
        for item in all_hid_device()?.into_iter() {
            if item.usage_page != 0xff00 {
                continue;
            }
            self.manager.add_devices(item.id, item)?;
        }
        let manager = self.manager.clone();
        let thread_handle =  spawn(move ||{
            let func = Box::new(move || {
                if let Err(err) = Self::usb_device_change(&manager) {
                    println!("usb 监听错误{:?}",err);
                }
            });
            let result = PnPDetectWindows::new(func);
            if let Err(e) = result.detect(){
                println!("热插拔注册错误：{:?}",e);
            }
        });
        let mut handle = self.thread_handle.lock().unwrap();
        *handle = Some(thread_handle);
        Ok(())
    }

    pub fn events(&self) -> Result<Receiver<CentralEvent>>{
        Ok(self.manager.event_channel())
    }

    pub fn peripherals(&self) -> Result<Vec<HidDevice>> {
        Ok(self.manager.devices())
    }

    pub fn peripheral(&self, id: &Uuid) -> Result<HidDevice> {
        self.manager.device(id).ok_or(Error::NotFound.into())
    }

    fn usb_device_change(manager: &Manager) -> Result<()>{
        let current_device = all_hid_device()?;
        let added_devices = current_device.iter().filter(|&u| (!manager.contains_device(u.id) && u.usage_page == 0xff00)).collect::<Vec<_>>();
        for item in added_devices.into_iter(){
            manager.add_devices(item.id,  item.clone())?;
            manager.emit(CentralEvent::DeviceAdd(item.id));
        }
        // 计算移除的设备 
        let new_key = current_device.iter().map(|d| d.id.clone()).collect::<Vec<_>>();
        let current_keys = manager.device_keys();
        let removed_keys = current_keys.iter().filter(|&u| !new_key.contains(u)).collect::<Vec<_>>();
        for key in removed_keys {
            match manager.remove_device(key.clone()) {
                Some((_, val)) => {
                    manager.emit(CentralEvent::DeviceRemove(val));
                }
                None => continue,
            }
        }
        Ok(())
    }
}   