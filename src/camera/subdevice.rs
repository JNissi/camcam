use std::{
    io,
    mem,
    path::Path,
    sync::Arc
};
use v4l::{v4l2};
use v4l_subdev::*;
use crate::camera::media_device::Handle;
use crate::camera::media_ioctl as ioctl;
use crate::camera::topology::*;

pub struct Subdevice {
    pub handle: Arc<Handle>,
    pub entity: Entity,
    pub interface: Interface,
    pub pad: Pad
}

impl Subdevice {
    pub fn open<P: AsRef<Path>>(path: P, entity: &Entity, interface: &Interface, pad: &Pad) -> io::Result<Self> {
        let fd = v4l2::open(&path, libc::O_RDWR)?;

        if fd == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(Subdevice {
            handle: Arc::new(Handle::new(fd)),
            entity: entity.clone(),
            interface: interface.clone(),
            pad: pad.clone()
        })
    }

    fn handle(&self) -> Arc<Handle> {
        self.handle.clone()
    }

    pub fn set_format(&self, width: u32, height: u32) {
        unsafe {
            let mut format: v4l2_subdev_format = mem::zeroed();
            format.which = 1;
            format.pad = 0;
            format.format.width = width;
            format.format.height = height;
            format.format.code = 0x3001; //<- BGGR 0x3014; <- RGGB
            format.format.field = 0;
            format.format.colorspace = v4l2_colorspace_V4L2_COLORSPACE_RAW;

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::VIDIOC_SUBDEV_S_FMT,
                &mut format as *mut _ as *mut std::os::raw::c_void
            ).expect("Failed setting subdevice format.");

            let format = SubdevFormat::from(&format);

            println!("Set subdevice format: {:#?}", format);
        }
    }

    pub fn set_interval(&self, numerator: u32, denominator: u32) {
        unsafe {
            let mut interval: v4l2_subdev_frame_interval = mem::zeroed();

            interval.interval.numerator = numerator;
            interval.interval.denominator = denominator;

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::VIDIOC_SUBDEV_G_FRAME_INTERVAL,
                &mut interval as *mut _ as *mut std::os::raw::c_void
            ).expect("Failed querying subdevice interval");

            let numerator = interval.interval.numerator;
            let denominator = interval.interval.denominator;

            println!("Set subdevice interval: {}/{}", numerator, denominator);
        }
    }

    pub fn auto_focus(&self, enable: bool) {
        unsafe {
            let mut val = v4l2_control {
                id: V4L2_CID_FOCUS_AUTO,
                value: if enable { 1 } else { 0 }
            };

            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_S_CTRL,
                &mut val as *mut _ as *mut std::os::raw::c_void
            ).expect("Failed setting subdev autofocus.");
        }
    }

    pub fn print_interval(&self) {
        unsafe {
            let mut interval: v4l2_subdev_frame_interval = mem::zeroed();

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::VIDIOC_SUBDEV_G_FRAME_INTERVAL,
                &mut interval as *mut _ as *mut std::os::raw::c_void
            ).expect("Failed querying subdevice interval");

            let numerator = interval.interval.numerator;
            let denominator = interval.interval.denominator;

            println!("Subdevice interval: {}/{}", numerator, denominator);
        }
    }

    pub fn print_format(&self) {
        unsafe {
            let mut format: v4l2_subdev_format = mem::zeroed();

            format.pad = 0;
            format.which = v4l2_subdev_format_whence_V4L2_SUBDEV_FORMAT_TRY;

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::VIDIOC_SUBDEV_G_FMT,
                &mut format as *mut _ as *mut std::os::raw::c_void
            ).expect("Failed reading subdevice format.");

            let format = SubdevFormat::from(&format);

            println!("Subdevice format: {:#?}", format);
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubdevFormat {
    pub which: u32,
    pub pad: u32,
    pub format: FrameFormat
}

impl From<&v4l2_subdev_format> for SubdevFormat {
    fn from(fmt: &v4l2_subdev_format) -> SubdevFormat {
        SubdevFormat {
            which: fmt.which,
            pad: fmt.pad,
            format: FrameFormat::from(&fmt.format)
        }
    }
}

#[derive(Clone, Debug)]
pub struct FrameFormat {
    pub width: u32,
    pub height: u32,
    pub code: u32,
    pub field: u32,
    pub colorspace: u32,
    pub quantization: u16,
    pub xfer_func: u16
}

impl From<&v4l2_mbus_framefmt> for FrameFormat {
    fn from(fmt: &v4l2_mbus_framefmt) -> FrameFormat {
        FrameFormat {
            width: fmt.width,
            height: fmt.height,
            code: fmt.code,
            field: fmt.field,
            colorspace: fmt.colorspace,
            quantization: fmt.quantization,
            xfer_func: fmt.xfer_func
        }
    }
}
