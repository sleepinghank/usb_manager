use std::{
    ffi::{OsString, c_void}, mem::size_of,
    sync::{atomic::{AtomicBool, Ordering}, Arc, RwLock}
};
use anyhow::{Result, bail};
use uuid::Uuid;
use windows::{
    Win32::{
        Storage::FileSystem::{
            CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, WriteFile, ReadFile
        },
        Devices::HumanInterfaceDevice::{
            HIDD_ATTRIBUTES,
            HIDP_CAPS,
            HidD_GetHidGuid, 
            HidD_GetPreparsedData,
            HidP_GetCaps,
            HidD_FreePreparsedData, 
            HidD_GetAttributes, HidD_SetOutputReport, HidD_GetInputReport, HidD_GetFeature, HidD_FlushQueue,
        },
        Foundation::{
            HANDLE,
            CloseHandle,
        }
    }
};

use super::{Error,utils::to_uuid, device_interface::DeviceInfoSet};

/// 1.获取所有设备，获取想要的设备信息
///
///     a.打开设备
///     b.关闭设备
///     c.获取设备报告描述符信息
///     d.获取设备属性
/// 2.根据获取到的设备进行读写操作
///
///     a.set_output_report /* Interrupt or Control*/
///     b.get_input_report /* Interrupt or Control*/
///     c.get_feature /* Interrupt or Control*/
///     d.read_file /*Interrupt*/
///     e.write_file /*Interrupt*/
///
#[derive(Debug,Default, Clone)]
pub struct HidDevice{
    pub id:Uuid,
    pub path:OsString,                                       //< stores the device's path. std::string             
    pub serial:String,                                    //< stores the device's serial number. std::wstring            
    pub manufacturer:String,                             //< stores the device's manufacturer. std::wstring            
    pub product:String,                                    //< stores the device's product string. std::wstring            
    pub vendor_id:u16,                                   //< stores the device's vendor id. unsigned short          
    pub product_id:u16,                                //< stores the device's product id. unsigned short          
    pub release:u16,                               //< stores the device's relase number. unsigned short          
    pub usage_page:u16,                                   //< stores the device's usage page. unsigned short          
    pub usage:u16,                                  //< stores the device's usage. unsigned short          
     // interface_number:u16,                           //< stores the device's interface number. int                     
    pub input_report_byte_length:u32,                    // 指定所有输入报告的最大大小（以字节为单位）。包括报表数据前面的报表 ID。如果未使用报表 ID，则 ID 值为零。      
    pub output_report_byte_length:u32,                   //< stores the device's write buffer size. unsigned short          
    pub feature_report_byte_length:u32,                   //< stores the device's write buffer size. unsigned short 
    //  readFifoBuffer;                              // internal read fifo buffer. 
    // *backgroundReader;                            // backgroud reader system. HidDeviceReaderThread   *
    device_handle: Arc<DeviceHandle>,
}

#[derive(Debug,Default)]
struct DeviceHandle {
    handle:RwLock<Option<HANDLE>>, // 打开该HID 设备的句柄 使用内部可变   
    opened:AtomicBool,             // stores the device file's status. mutable bool       使用内部可变     
}


/// 销毁时关闭该设备 
impl Drop for HidDevice {
    fn drop(&mut self) {
        let handle_read = match self.device_handle.handle.read() {
            Ok(v) => v,
            Err(_) => return,
        };
        
        match *handle_read {
            Some(handle) => {
                if unsafe { !CloseHandle(handle) }.as_bool() {
                    // println!("{}", Error::win32());
                }
            },
            None => return,
        }
    }
}

impl HidDevice {

    pub fn new(id:Uuid,path:OsString) -> Self {
        let mut device = Self::default();
        device.path = path;
        device.id = id;
        device
    }

    /// 打开设备
    fn open_device(&self) -> Result<HANDLE> {
        unsafe {
            let device_handle  = CreateFileW(
                self.path.clone(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE, 
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(), 
                OPEN_EXISTING, 
                FILE_ATTRIBUTE_NORMAL, 
                windows::Win32::Foundation::HANDLE::default())?;
            if device_handle.is_invalid(){
                bail!(Error::OpenError);
            }
            let mut handle_mut = self.device_handle.handle.write().unwrap();
            *handle_mut = Some(device_handle.clone());
            self.device_handle.opened.store(true, Ordering::Relaxed);
            Ok(device_handle)
        }
    }

    /// 关闭当前设备 
    pub fn close_device(&self) -> bool {
        match *self.device_handle.handle.read().unwrap() {
            Some(handle) => {
                if unsafe { !CloseHandle(handle) }.as_bool() {
                    // println!("{}", Error::win32());
                }
            },
            None => {},
        }
        let mut handle_read = self.device_handle.handle.write().unwrap();
        *handle_read = None;
        self.device_handle.opened.store(false, Ordering::Relaxed);
        true
    }

    /// 获取设备报告描述符信息
    fn get_usage_info(&mut self) -> Result<()> {
        let handle = self.device_handle.handle.read().unwrap().
            ok_or(Error::NotOpen)?;
        unsafe {
            let mut pp_data:isize = 0;
            let mut cpas = HIDP_CAPS::default();
            if HidD_GetPreparsedData(handle,&mut pp_data).0 == 1{
                if let Err(err) = HidP_GetCaps(pp_data,&mut cpas){
                    bail!(err);
                }
                HidD_FreePreparsedData(pp_data);
            } else {
                bail!(Error::win32());
            }
            self.usage_page = cpas.UsagePage;
            self.usage = cpas.Usage;
            self.input_report_byte_length = cpas.InputReportByteLength as u32;
            self.output_report_byte_length = cpas.OutputReportByteLength as u32;
            self.feature_report_byte_length = cpas.FeatureReportByteLength as u32;
        }
        Ok(())
    }

    /// 获取设备属性
    fn get_attributes_info(&mut self) -> Result<()> {
        let handle = self.device_handle.handle.read().unwrap().
            ok_or(Error::NotOpen)?;
        unsafe {
            let mut attributes = HIDD_ATTRIBUTES {
                Size: size_of::<HIDD_ATTRIBUTES>() as u32,
                ..std::mem::zeroed()
            };
            if HidD_GetAttributes(handle, &mut attributes).0 == 0 {
                bail!(Error::win32());
            }
            self.vendor_id = attributes.VendorID;
            self.product_id = attributes.ProductID;
            self.release = attributes.VersionNumber;
        }
        Ok(())
    }

    /// 获取设备所有信息 
    fn get_device_info(&mut self) -> Result<()> {
        self.open_device()?;
        self.get_usage_info().map_err(|e| {self.close_device(); return e})?;
        self.get_attributes_info().map_err(|e| {self.close_device(); return e})?;
        self.close_device();
        Ok(())
    }

    /// 设置output数据 
    pub fn set_output_report(&self,report_id:u8, data:&[u8]) -> Result<()>{
        if (data.len() + 1) as u32 > self.output_report_byte_length{
            bail!(Error::DataOverlength);
        }
        let handle = self.check_handle()?;
        let send_data = self.output_assemble_data(report_id, data,self.output_report_byte_length as usize)?;
        if unsafe{HidD_SetOutputReport(handle, send_data.as_ptr() as *const c_void,self.output_report_byte_length)}.0 == 0 {
            bail!(Error::win32());
        }
        self.close_device();
        Ok(())
    }


    /// 获取input数据 
    pub fn get_input_report(&self,report_id:u8, data_len:usize) -> Result<Vec<u8>>{
        if (data_len + 1)as u32 > self.input_report_byte_length{
            bail!(Error::DataOverlength);
        }
        let handle = self.check_handle()?;
        let mut send_data = self.input_assemble_data(report_id, self.input_report_byte_length as usize)?;
        if unsafe{HidD_GetInputReport(handle, send_data.as_mut_ptr() as *mut c_void,self.input_report_byte_length)}.0 == 0 {
            bail!(Error::win32());
        }
        self.close_device();
        if send_data[0] == report_id{
            send_data.remove(0);
        } 
        send_data.truncate(data_len);
        Ok(send_data)
    }

    /// 获取 feature数据 
    pub fn get_feature_report(&self,report_id:u8,data_len:usize) -> Result<Vec<u8>>{
        if (data_len + 1) as u32 > self.feature_report_byte_length{
            bail!(Error::DataOverlength);
        }
        let handle = self.check_handle()?;
        let mut send_data = self.input_assemble_data(report_id, self.feature_report_byte_length as usize)?;
        if unsafe{HidD_GetFeature(handle, send_data.as_mut_ptr() as *mut c_void,self.feature_report_byte_length)}.0 == 0 {
            bail!(Error::win32());
        }
        self.close_device();
        if send_data[0] == report_id{
            send_data.remove(0);
        }
        send_data.truncate(data_len);
        Ok(send_data)
    }

    /// 写入，可以异步
    pub fn write(&self,report_id:u8, data:&[u8]) -> Result<u32>{
        if (data.len() + 1) as u32 > self.output_report_byte_length {
            bail!(Error::DataOverlength);
        }
        let mut write_len:u32 = 0;
        let handle = self.check_handle()?;
        let send_data = self.output_assemble_data(report_id, data, self.output_report_byte_length as usize)?;
        if !unsafe{WriteFile(handle, send_data.as_ptr() as *const c_void,self.output_report_byte_length,&mut write_len,std::ptr::null_mut())}.as_bool(){
            bail!(Error::win32());
        }
        self.close_device();
        if write_len <= 0 {
            bail!("write error");
        }
        Ok(write_len)
    }

    /// 读取
    pub fn read(&self,report_id:u8, data_len:usize) -> Result<Vec<u8>>{
        // self.read_flush()?;
        let read_data= self.read_continuous(report_id, data_len)?;
        self.close_device();
        Ok(read_data)
    }

    /// 刷新读缓冲区
    pub fn read_flush(&self) -> Result<()>{
        let handle = self.check_handle()?;
        if unsafe {HidD_FlushQueue(handle)}.0 == 0{
            println!("Failed to flush the read buffer")
        }
        Ok(())
    }

    /// 读取 连续的
    pub fn read_continuous(&self,report_id:u8, data_len: usize) -> Result<Vec<u8>>{
        if (data_len + 1) as u32 > self.input_report_byte_length{
            bail!(Error::DataOverlength);
        }
        let mut read_len:u32 = 0;
        let handle = self.check_handle()?;
        let mut send_data = self.input_assemble_data(report_id, self.input_report_byte_length as usize)?;
        if !unsafe{ReadFile(handle, send_data.as_mut_ptr() as *mut c_void,self.input_report_byte_length,&mut read_len,std::ptr::null_mut())}.as_bool(){
            bail!(Error::win32());
        }
        if read_len <= 0 {
            bail!("read error");
        }
        if send_data[0] == report_id{
            send_data.remove(0);
        }
        send_data.truncate(data_len);
        Ok(send_data)
    }

    /// 组装 output 数据
    fn output_assemble_data(&self, report_id: u8, data: &[u8],data_len: usize) -> Result<Vec<u8>> {
        let mut send_data: Vec<u8> = data.into_iter().map(|&x| x.clone()).collect();
        send_data.reverse();
        send_data.push(report_id);
        send_data.reverse();
        if send_data.len() < data_len{
            send_data.append(&mut vec![0u8;data_len - send_data.len()])
        }
        Ok(send_data)
    }

    /// 组装 input 数据
    fn input_assemble_data(&self, report_id: u8, data_len: usize) -> Result<Vec<u8>> {
        let mut read_data: Vec<u8> = vec![0; data_len];
        read_data[0] = report_id;
        Ok(read_data)
    }

    /// 检查设备句柄
    fn check_handle(&self) -> Result<HANDLE> {
        let read_handle = *self.device_handle.handle.read().unwrap();
        let handle = match read_handle{
            Some(v) => v,
            None => self.open_device()?,
        };
        Ok(handle)
    }
}

    /// 获取所有的 hid 设备
pub fn all_hid_device() -> Result<Vec<HidDevice>> {
    let mut list = vec![];
    // 1.获取 hid GUID 
    let mut p_guid = ::windows::core::GUID::new()?;
    unsafe {HidD_GetHidGuid(&mut p_guid)}
    // 2.根据 HID GUID 获取HID 设备列表
    let device_info_set = DeviceInfoSet::new(Some(&p_guid))?;
    for (device_interface_name, device) in
    device_info_set.iter_device_interfaces(p_guid){
        let id = device_info_set.get_container_id(&device)?;
        let mut device_info = HidDevice::new(to_uuid(&id),device_interface_name);
        if let Err(_err) = device_info.get_device_info() {
            continue;
        }
        list.push(device_info)
    }
    Ok(list)
}


#[cfg(test)]
mod tests {

    use crate::{hid_device::{HidDevice,all_hid_device}};
    #[test]                     
    fn set_output_report_test() {
        // for device in all_hid_device().unwrap() {
        //     let data = vec![1;64];
        //     if let Err(e) = device.set_output_report(0x00, data.as_slice()) {
        //         println!("err {}",e);
        //     }
        //     device.close_device();
        //     break;
        // }
        let device = all_hid_device().unwrap().into_iter().find(|x| x.input_report_byte_length == 65).unwrap();
        let data = vec![1;2];
        device.set_output_report(0x00, data.as_slice()).unwrap();
        assert_eq!(1, 1);
    }

    #[test]
    fn get_input_report_test() {
        let device = all_hid_device().unwrap().into_iter().find(|x| x.input_report_byte_length == 65).unwrap();
        // let mut device = all_hid_device().unwrap().pop().unwrap();
        let result = device.get_input_report(0x00, 51).unwrap();
        println!("{:?}", result);
        assert_eq!(result.len(), 51);
    }


    #[test]
    fn get_feature_test() {
        let device = all_hid_device().unwrap().into_iter().find(|x| x.feature_report_byte_length == 65).unwrap();
        let result = device.get_feature_report(0x00, 51).unwrap();
        println!("result:{:?}", result);
        assert_eq!(result.len(), 51);
    }

    #[test]
    fn write_test() {
        let device = all_hid_device().unwrap().into_iter().find(|x| x.feature_report_byte_length == 65).unwrap();
        let data = [1;64];
        let write_len = device.write(0x00, &data).unwrap();
        println!("write_len:{:?}", write_len);
        // device.close_device();
        assert_eq!(write_len, 64);
    }

    #[test]
    fn read_test() {
        let device = all_hid_device().unwrap().into_iter().find(|x| x.output_report_byte_length == 65).unwrap();
        let result =device.read(0x00, 64).unwrap();
        println!("result:{:?}", result);
        // device.close_device();
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn read_continuous_test() {
        let device = all_hid_device().unwrap().into_iter().find(|x| x.input_report_byte_length == 65).unwrap();
        println!("input_report_byte_length:{}", device.input_report_byte_length);
        for i in 1..10 {
            println!("{}",i);
            let result =device.read_continuous(0x00, 64).unwrap();
            println!("result:{:?}", result);
        }
        device.close_device();
        assert_eq!(1, 1);
    }
}