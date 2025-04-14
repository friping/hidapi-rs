use crate::windows_native::error::{WinError, WinResult};
use crate::BusType;
use std::mem::{size_of, zeroed};
use std::ptr::null;
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::Properties::{DEVPROPKEY, DEVPROPTYPE, DEVPROP_TYPE_GUID};
use windows_sys::Win32::Foundation::{
    CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE, TRUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

#[allow(clippy::missing_safety_doc)]
pub unsafe trait DeviceProperty {
    const TYPE: DEVPROPTYPE;
    fn create_sized(bytes: usize) -> Self;
    fn as_ptr_mut(&mut self) -> *mut u8;
    fn validate(&self) {}
}

unsafe impl DeviceProperty for GUID {
    const TYPE: DEVPROPTYPE = DEVPROP_TYPE_GUID;

    fn create_sized(bytes: usize) -> Self {
        assert_eq!(bytes, size_of::<GUID>());
        GUID::from_u128(0)
    }

    fn as_ptr_mut(&mut self) -> *mut u8 {
        (self as *mut GUID) as *mut u8
    }
}

pub trait PropertyKey: Copy {
    fn as_ptr(&self) -> *const DEVPROPKEY;
}

impl PropertyKey for DEVPROPKEY {
    fn as_ptr(&self) -> *const DEVPROPKEY {
        self
    }
}

impl PropertyKey for PROPERTYKEY {
    fn as_ptr(&self) -> *const DEVPROPKEY {
        self as *const PROPERTYKEY as _
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InternalBusType {
    Unknown,
    Usb,
    Bluetooth,
    BluetoothLE,
    I2c,
    Spi,
}

impl From<InternalBusType> for BusType {
    fn from(value: InternalBusType) -> Self {
        match value {
            InternalBusType::Unknown => BusType::Unknown,
            InternalBusType::Usb => BusType::Usb,
            InternalBusType::Bluetooth => BusType::Bluetooth,
            InternalBusType::BluetoothLE => BusType::Bluetooth,
            InternalBusType::I2c => BusType::I2c,
            InternalBusType::Spi => BusType::Spi,
        }
    }
}

pub struct Handle(HANDLE);

impl Handle {
    pub fn from_raw(handle: HANDLE) -> Self {
        Self(handle)
    }
    pub fn as_raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
        self.0 = INVALID_HANDLE_VALUE;
    }
}

pub struct Overlapped(OVERLAPPED);

impl Overlapped {
    pub fn event_handle(&self) -> HANDLE {
        self.0.hEvent
    }
    pub fn as_raw(&mut self) -> *mut OVERLAPPED {
        &mut self.0
    }

    pub fn get_result(&mut self, handle: &Handle, timeout: Option<u32>) -> WinResult<usize> {
        let mut bytes_written = 0;
        let overlapped = self.as_raw();

        // 通过事件对象实现超时等待（网页1/网页6）
        let wait_result = unsafe {
            WaitForSingleObject(
                (*overlapped).hEvent, // 使用OVERLAPPED结构中的事件
                timeout.unwrap_or(INFINITE),
            )
        };

        // 处理等待结果（网页2/网页5）
        ensure!(
            wait_result == WAIT_OBJECT_0,
            Err(WinError::from_win32(unsafe { GetLastError() }))
        );

        // 获取I/O操作结果（网页3/网页7）
        let cr = unsafe {
            GetOverlappedResult(
                handle.as_raw(),
                overlapped,
                &mut bytes_written,
                FALSE, // 非阻塞模式，因已通过事件等待完成
            )
        };

        ensure!(cr == TRUE, Err(WinError::last()));
        Ok(bytes_written as usize)
    }
}

unsafe impl Send for Overlapped {}

impl Default for Overlapped {
    fn default() -> Self {
        Overlapped(unsafe {
            OVERLAPPED {
                //todo check if event is null
                hEvent: CreateEventW(null(), FALSE, FALSE, null()),
                ..zeroed()
            }
        })
    }
}

impl Drop for Overlapped {
    fn drop(&mut self) {
        if self.0.hEvent != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0.hEvent);
            }
        }
        self.0.hEvent = INVALID_HANDLE_VALUE;
    }
}
