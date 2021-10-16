#![allow(dead_code)] // A lot of the consts are unused

use linux_media::*;
use v4l_subdev::*;

#[allow(non_camel_case_types)]
pub type _IOC_TYPE = std::os::raw::c_ulong;

// linux ioctl.h
const _IOC_NRBITS: u8 = 8;
const _IOC_TYPEBITS: u8 = 8;

const _IOC_SIZEBITS: u8 = 14;

const _IOC_NRSHIFT: u8 = 0;
const _IOC_TYPESHIFT: u8 = _IOC_NRSHIFT + _IOC_NRBITS;
const _IOC_SIZESHIFT: u8 = _IOC_TYPESHIFT + _IOC_TYPEBITS;
const _IOC_DIRSHIFT: u8 = _IOC_SIZESHIFT + _IOC_SIZEBITS;

const _IOC_NONE: u8 = 0;
const _IOC_WRITE: u8 = 1;
const _IOC_READ: u8 = 2;

macro_rules! _IOC_TYPECHECK {
    ($type:ty) => {
        std::mem::size_of::<$type>()
    };
}

macro_rules! _IOC {
    ($dir:expr, $type:expr, $nr:expr, $size:expr) => {
        (($dir as _IOC_TYPE) << $crate::camera::media_ioctl::_IOC_DIRSHIFT)
            | (($type as _IOC_TYPE) << $crate::camera::media_ioctl::_IOC_TYPESHIFT)
            | (($nr as _IOC_TYPE) << $crate::camera::media_ioctl::_IOC_NRSHIFT)
            | (($size as _IOC_TYPE) << $crate::camera::media_ioctl::_IOC_SIZESHIFT)
    };
}

macro_rules! _IO {
    ($type:expr, $nr:expr) => {
        _IOC!($crate::camera::media_ioctl::_IOC_NONE, $type, $nr, 0)
    };
}

macro_rules! _IOR {
    ($type:expr, $nr:expr, $size:ty) => {
        _IOC!(
            $crate::camera::media_ioctl::_IOC_READ,
            $type,
            $nr,
            _IOC_TYPECHECK!($size)
        )
    };
}

macro_rules! _IOW {
    ($type:expr, $nr:expr, $size:ty) => {
        _IOC!(
            $crate::camera::media_ioctl::_IOC_WRITE,
            $type,
            $nr,
            _IOC_TYPECHECK!($size)
        )
    };
}

macro_rules! _IOWR {
    ($type:expr, $nr:expr, $size:ty) => {
        _IOC!(
            $crate::camera::media_ioctl::_IOC_READ | $crate::camera::media_ioctl::_IOC_WRITE,
            $type,
            $nr,
            _IOC_TYPECHECK!($size)
        )
    };
}

//linux/media.h
pub const MEDIA_IOC_DEVICE_INFO:   _IOC_TYPE = _IOWR!(b'|', 0x00, media_device_info);
pub const MEDIA_IOC_ENUM_ENTITIES: _IOC_TYPE = _IOWR!(b'|', 0x01, media_entity_desc);
pub const MEDIA_IOC_ENUM_LINKS:    _IOC_TYPE = _IOWR!(b'|', 0x02, media_links_enum);
pub const MEDIA_IOC_SETUP_LINK:    _IOC_TYPE = _IOWR!(b'|', 0x03, media_link_desc);
pub const MEDIA_IOC_G_TOPOLOGY:    _IOC_TYPE = _IOWR!(b'|', 0x04, media_v2_topology);


//linux/v4l-subdev.h
pub const VIDIOC_SUBDEV_QUERYCAP:            _IOC_TYPE =  _IOR!(b'V', 0,  v4l2_subdev_capability);
pub const VIDIOC_SUBDEV_G_FMT:               _IOC_TYPE = _IOWR!(b'V', 4,  v4l2_subdev_format);
pub const VIDIOC_SUBDEV_S_FMT:               _IOC_TYPE = _IOWR!(b'V', 5,  v4l2_subdev_format);
pub const VIDIOC_SUBDEV_G_FRAME_INTERVAL:    _IOC_TYPE = _IOWR!(b'V', 21, v4l2_subdev_frame_interval);
pub const VIDIOC_SUBDEV_S_FRAME_INTERVAL:    _IOC_TYPE = _IOWR!(b'V', 22, v4l2_subdev_frame_interval);
pub const VIDIOC_SUBDEV_ENUM_MBUS_CODE:      _IOC_TYPE = _IOWR!(b'V', 2,  v4l2_subdev_mbus_code_enum);
pub const VIDIOC_SUBDEV_ENUM_FRAME_SIZE:     _IOC_TYPE = _IOWR!(b'V', 74, v4l2_subdev_frame_size_enum);
pub const VIDIOC_SUBDEV_ENUM_FRAME_INTERVAL: _IOC_TYPE = _IOWR!(b'V', 75, v4l2_subdev_frame_interval_enum);
pub const VIDIOC_SUBDEV_G_CROP:              _IOC_TYPE = _IOWR!(b'V', 59, v4l2_subdev_crop);
pub const VIDIOC_SUBDEV_S_CROP:              _IOC_TYPE = _IOWR!(b'V', 60, v4l2_subdev_crop);
pub const VIDIOC_SUBDEV_G_SELECTION:         _IOC_TYPE = _IOWR!(b'V', 61, v4l2_subdev_selection);
pub const VIDIOC_SUBDEV_S_SELECTION:         _IOC_TYPE = _IOWR!(b'V', 62, v4l2_subdev_selection);
