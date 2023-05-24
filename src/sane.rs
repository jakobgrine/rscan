use std::ffi::{CStr, CString};

use anyhow::{anyhow, Result};
use image::{Luma, Rgb};
use sane_scan::{sys, Device, Frame, Sane, DeviceHandle};

pub(crate) trait GetAllDevices {
    fn get_all_devices(&self) -> Result<Vec<Device>, sane_scan::Error>;
}

fn cstring_from_ptr(ptr: *const std::os::raw::c_char) -> CString {
    if ptr.is_null() {
        return CString::default();
    }
    unsafe { CStr::from_ptr(ptr).to_owned() }
}

impl GetAllDevices for Sane {
    fn get_all_devices(&self) -> Result<Vec<Device>, sane_scan::Error> {
        let mut device_list: *mut *const sys::Device = std::ptr::null_mut();
        let status: sys::Status =
            unsafe { sys::sane_get_devices(&mut device_list as *mut *mut *const sys::Device, 0) };
        if status != sys::Status::Good {
            return Err(sane_scan::Error(status));
        }
        let device_count = unsafe {
            let mut device_count = 0_usize;
            while !(*device_list.add(device_count)).is_null() {
                device_count += 1;
            }
            device_count
        };
        let device_list: &[*const sys::Device] =
            unsafe { std::slice::from_raw_parts(device_list, device_count) };

        let device_list: Vec<Device> = unsafe {
            device_list
                .iter()
                .copied()
                .map(|device| {
                    let name = cstring_from_ptr((*device).name);
                    let vendor = cstring_from_ptr((*device).vendor);
                    let model = cstring_from_ptr((*device).model);
                    let type_ = cstring_from_ptr((*device).type_);
                    Device {
                        name,
                        vendor,
                        model,
                        type_,
                    }
                })
                .collect()
        };
        Ok(device_list)
    }
}

pub(crate) enum ImageBuffer {
    Luma(image::ImageBuffer<Luma<u8>, Vec<u8>>),
    Rgb(image::ImageBuffer<Rgb<u8>, Vec<u8>>),
}

pub(crate) trait ReadToImage {
    fn read_to_image(&mut self) -> Result<ImageBuffer>;
}

impl ReadToImage for DeviceHandle {
    fn read_to_image(&mut self) -> Result<ImageBuffer> {
        let parameters = self.get_parameters()?;
        let image_data = self.read_to_vec()?;
        let image = if parameters.format == Frame::Gray {
            let buffer = image::ImageBuffer::<Luma<_>, _>::from_raw(
                parameters.pixels_per_line as u32,
                parameters.lines as u32,
                image_data,
            )
            .ok_or(anyhow!("failed to parse Gray image"))?;
            ImageBuffer::Luma(buffer)
        } else {
            let buffer = image::ImageBuffer::<Rgb<_>, _>::from_raw(
                parameters.pixels_per_line as u32,
                parameters.lines as u32,
                image_data,
            )
            .ok_or(anyhow!("failed to parse Rgb image"))?;
            ImageBuffer::Rgb(buffer)
        };
        Ok(image)
    }
}
