use lazy_static::lazy_static;
use linux_media::*;
use v4l_subdev::*;
use regex::Regex;
use std::path::PathBuf;
use std::str;
use std::os::raw::c_char;
use std::{alloc::{alloc_zeroed, Layout}, ffi::CStr, fs, io, mem, path::Path, slice, sync::Arc};
use v4l::{v4l2};
use crate::camera::media_ioctl as ioctl;

pub struct MediaDevice {
    handle: Arc<Handle>,
    video_device: Option<VideoDevice>,
    front_camera: Option<Subdevice>,
    back_camera: Option<Subdevice>
}

pub struct VideoDevice {
    // No need for handle, that's handled (pun intented) on Camera level.
    entity: Entity,
    interface: Interface,
    pad: Pad
}

impl VideoDevice {
    pub fn new(entity: &Entity, interface: &Interface, pad: &Pad) -> Self {
        VideoDevice {
            entity: entity.clone(),
            interface: interface.clone(),
            pad: pad.clone()
        }
    }
}

pub struct Subdevice {
    handle: Arc<Handle>,
    entity: Entity,
    interface: Interface,
    pad: Pad
}

impl Subdevice {
    pub fn open<P: AsRef<Path>>(path: P, entity: &Entity, interface: &Interface, pad: &Pad) -> io::Result<Self> {
        let fd = v4l2::open(&path, libc::O_RDWR)?;

        if fd == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(Subdevice {
            handle: Arc::new(Handle { fd }),
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
            format.format.width = width;
            format.format.height = height;
            format.format.code = 0x3001; //<- BGGR 0x3014; <- RGGB
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

impl MediaDevice {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let fd = v4l2::open(&path, libc::O_RDWR)?;

        if fd == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(MediaDevice {
            handle: Arc::new(Handle { fd }),
            video_device: None,
            front_camera: None,
            back_camera: None
        })
    }

    fn handle(&self) -> Arc<Handle> {
        self.handle.clone()
    }

    pub fn info(&self) -> io::Result<Info> {
        unsafe {
            let mut device_info: media_device_info = mem::zeroed();

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::MEDIA_IOC_DEVICE_INFO,
                &mut device_info as *mut _ as *mut std::os::raw::c_void
            )?;

            Ok(Info::from(device_info))
        }
    }

    pub fn setup(&mut self) {
        let topology = self.topology().expect("Couldn't get topology.");

        let entities = topology.entities;

        let mut back_camera_entity = None;
        let mut front_camera_entity = None;
        let mut video_entity = None;


        entities.iter().for_each(|e| {
            match &e.name[0..6] {
                "ov5640" => back_camera_entity = Some(e.clone()),
                "gc2145" => front_camera_entity = Some(e.clone()),
                "sun6i-" => video_entity = Some(e.clone()),
                _ => println!("Unknown entity name: {}", e.name)
            }
        });

        let interfaces = topology.interfaces;
        let pads = topology.pads;
        let links = topology.links;

        let back_camera_entity = back_camera_entity.expect("Back camera entity not set!");
        let back_camera_pad = pads.iter().find(|p| p.entity_id == back_camera_entity.id)
            .expect("Can't find pad for back camera sensor.");

        let back_camera_interface_link = links.iter().find(|l| l.sink_id == back_camera_entity.id)
            .expect("Can't find link between back camera entity and interface.");

        let back_camera_interface = interfaces.iter().find(|i| i.id == back_camera_interface_link.source_id)
            .expect("Can't find back camera interface.");

        let path = get_device_path_from_interface(&back_camera_interface);

        let back_camera = Subdevice::open(
            &path,
            &back_camera_entity,
            back_camera_interface,
            back_camera_pad
        ).expect("Can't open back camera.");

        self.back_camera = Some(back_camera);

        let front_camera_entity = front_camera_entity.expect("front camera entity not set!");
        let front_camera_pad = pads.iter().find(|p| p.entity_id == front_camera_entity.id)
            .expect("Can't find pad for front camera sensor.");

        let front_camera_interface_link = links.iter().find(|l| l.sink_id == front_camera_entity.id)
            .expect("Can't find link between front camera entity and interface.");

        let front_camera_interface = interfaces.iter().find(|i| i.id == front_camera_interface_link.source_id)
            .expect("Can't find front camera interface.");

        let path = get_device_path_from_interface(&front_camera_interface);

        let front_camera = Subdevice::open(
            &path,
            &front_camera_entity,
            front_camera_interface,
            front_camera_pad
        ).expect("Can't open front camera.");

        self.front_camera = Some(front_camera);

        let video_entity = video_entity.expect("Couldn't get video entity.");
        let video_pad = pads.iter().find(|p| p.entity_id == video_entity.id)
            .expect("Can't find video device pad.");
        let video_interface_link = links.iter().find(|l| l.sink_id == video_entity.id)
            .expect("Can't find video device interface link.");
        let video_interface = interfaces.iter().find(|i| i.id == video_interface_link.source_id)
            .expect("Can't find video device interface.");

        let video_device = VideoDevice::new(
            &video_entity,
            &video_interface,
            &video_pad
        );
        self.video_device = Some(video_device);

        self.unlink_back_camera();
        self.unlink_front_camera();
        self.link_front_camera();
    }

    pub fn set_format(&self, width: u32, height: u32) {
        self.back_camera.as_ref().unwrap().set_format(width, height);
    }

    pub fn set_interval(&self, numerator: u32, denominator: u32) {
        self.back_camera.as_ref().unwrap().set_interval(numerator, denominator);
    }

    pub fn auto_focus(&self, enable: bool) {
        self.back_camera.as_ref().unwrap().auto_focus(enable);
    }

    pub fn topology(&self) -> io::Result<Topology> {
        unsafe {
            let mut topology: media_v2_topology = mem::zeroed();

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::MEDIA_IOC_G_TOPOLOGY,
                &mut topology as *mut _ as *mut std::os::raw::c_void
            )?;

            let entity_count = topology.num_entities as usize;
            let interface_count = topology.num_interfaces as usize;
            let pad_count = topology.num_pads as usize;
            let link_count = topology.num_links as usize;

            let mut entities = Layout::array::<media_v2_entity>(entity_count)
                .expect("Couldn't allocate memory for entities.");
            topology.ptr_entities = alloc_zeroed(entities) as u64;
            let mut interfaces = Layout::array::<media_v2_interface>(interface_count)
                .expect("Couldn't allocate memory for interfaces");
            topology.ptr_interfaces = alloc_zeroed(interfaces) as u64;
            let mut pads = Layout::array::<media_v2_pad>(pad_count)
                .expect("Couldn't allocate memory for pads.");
            topology.ptr_pads = alloc_zeroed(pads) as u64;
            let mut links = Layout::array::<media_v2_link>(link_count)
                .expect("Couldn't allocate memory for links.");
            topology.ptr_links = alloc_zeroed(links) as u64;

            v4l2::ioctl(
                self.handle().fd(),
                ioctl::MEDIA_IOC_G_TOPOLOGY,
                &mut topology as *mut _ as *mut std::os::raw::c_void
            )?;

            let entities = slice::from_raw_parts::<media_v2_entity>(topology.ptr_entities as *const media_v2_entity, entity_count);
            let interfaces = slice::from_raw_parts::<media_v2_interface>(topology.ptr_interfaces as *const media_v2_interface, interface_count);
            let pads = slice::from_raw_parts::<media_v2_pad>(topology.ptr_pads as *const media_v2_pad, pad_count);
            let links = slice::from_raw_parts::<media_v2_link>(topology.ptr_links as *const media_v2_link, link_count);

            Ok(Topology::from(topology, &entities, &interfaces, &pads, &links))
        }
    }

    pub fn link_back_camera(&self) {
        let cam_dev = self.back_camera.as_ref().expect("Back camera not set.");
        let video_dev = self.video_device.as_ref().expect("Video device not set.");
        self.setup_link(cam_dev.entity.id, video_dev.entity.id, true)
            .expect("Can't link back camera.");
    }

    pub fn unlink_back_camera(&self) {
        let cam_dev = self.back_camera.as_ref().expect("Back camera not set.");
        let video_dev = self.video_device.as_ref().expect("Video device not set.");
        self.setup_link(cam_dev.entity.id, video_dev.entity.id, false)
            .expect("Can't unlink back camera.");
    }

    pub fn link_front_camera(&self) {
        let cam_dev = self.front_camera.as_ref().expect("Front camera not set.");
        let video_dev = self.video_device.as_ref().expect("Video device not set.");
        self.setup_link(cam_dev.entity.id, video_dev.entity.id, true)
            .expect("Can't link front camera.");
        cam_dev.set_format(1280, 960);
        cam_dev.set_interval(1, 15);
    }

    pub fn unlink_front_camera(&self) {
        let cam_dev = self.front_camera.as_ref().expect("Front camera not set.");
        let video_dev = self.video_device.as_ref().expect("Video device not set.");
        self.setup_link(cam_dev.entity.id, video_dev.entity.id, false)
            .expect("Can't unlink front camera.");
    }

    fn setup_link(&self, source_id: u32, sink_id: u32, enable: bool) -> io::Result<()> {
        let flags = if enable {
            MEDIA_LNK_FL_ENABLED
        } else {
            0
        };
        let mut link = media_link_desc {
            source: media_pad_desc {
                entity: source_id,
                index: 0,
                flags: 0,
                reserved: [0; 2]
            },
            sink: media_pad_desc {
                entity: sink_id,
                index: 0,
                flags: 0,
                reserved: [0; 2]
            },
            flags: flags,
            reserved: [0; 2]
        };

        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                ioctl::MEDIA_IOC_SETUP_LINK,
                &mut link as *mut _ as *mut std::os::raw::c_void
            )?;
        }

        Ok(())
    }
}

struct Handle {
    fd: std::os::raw::c_int
}

impl Handle {
    pub fn fd(&self) -> std::os::raw::c_int {
        self.fd
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        v4l2::close(self.fd).unwrap();
    }
}

#[derive(Debug)]
pub struct Info {
    pub driver: String,
    pub model: String,
    pub serial: String,
    pub bus_info: String,
    pub media_version: (u8, u8, u8),
    pub hw_revision: u32,
    pub driver_version: (u8, u8, u8)
}

impl From<media_device_info> for Info {
    fn from(info: media_device_info) -> Info {
        Info {
            driver: c_char_array_to_string(&info.driver),
            model: c_char_array_to_string(&info.model),
            serial: c_char_array_to_string(&info.serial),
            bus_info: c_char_array_to_string(&info.bus_info),
            media_version: parse_kernel_version(info.media_version),
            hw_revision: info.hw_revision,
            driver_version: parse_kernel_version(info.driver_version)
        }
    }
}

#[derive(Debug)]
pub struct Topology {
    version: u64,
    entities: Vec<Entity>,
    interfaces: Vec<Interface>,
    pads: Vec<Pad>,
    links: Vec<Link>,
}

impl Topology {
    fn from(topology: media_v2_topology, entities: &[media_v2_entity], interfaces: &[media_v2_interface], pads: &[media_v2_pad], links: &[media_v2_link]) -> Topology {
        let entities = entities.iter().map(|e| Entity::from(e)).collect();
        let interfaces = interfaces.iter().map(|i| Interface::from(i)).collect();
        let pads = pads.iter().map(|p| Pad::from(p)).collect();
        let links = links.iter().map(|l| Link::from(l)).collect();
        Topology {
            version: topology.topology_version,
            entities: entities,
            interfaces: interfaces,
            pads: pads,
            links: links
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    id: u32,
    name: String,
    function: u32,
    flags: u32,
}

impl From<&media_v2_entity> for Entity {
    fn from(entity: &media_v2_entity) -> Entity {
        Entity {
            id: entity.id,
            name: c_char_array_to_string(&entity.name),
            function: entity.function,
            flags: entity.flags
        }
    }
}

#[derive(Clone, Debug)]
pub struct Interface {
    id: u32,
    interface_type: u32,
    flags: u32,
    major: u32,
    minor: u32
}

impl From<&media_v2_interface> for Interface {
    fn from(interface: &media_v2_interface) -> Interface {
        let (major, minor) = unsafe {
            (interface.__bindgen_anon_1.devnode.major,
            interface.__bindgen_anon_1.devnode.minor)
        };
        Interface {
            id: interface.id,
            interface_type: interface.intf_type,
            flags: interface.flags,
            major: major,
            minor: minor
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pad {
    id: u32,
    entity_id: u32,
    flags: u32,
    index: u32,
}

impl From<&media_v2_pad> for Pad {
    fn from(pad: &media_v2_pad) -> Pad {
        Pad {
            id: pad.id,
            entity_id: pad.entity_id,
            flags: pad.flags,
            index: pad.index
        }
    }
}

#[derive(Clone, Debug)]
pub struct Link {
    id: u32,
    source_id: u32,
    sink_id: u32,
    flags: u32,
}

impl From<&media_v2_link> for Link {
    fn from(link: &media_v2_link) -> Link {
        Link {
            id: link.id,
            source_id: link.source_id,
            sink_id: link.sink_id,
            flags: link.flags
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubdevFormat {
    which: u32,
    pad: u32,
    format: FrameFormat
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
    width: u32,
    height: u32,
    code: u32,
    field: u32,
    colorspace: u32,
    quantization: u16,
    xfer_func: u16
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

fn c_char_array_to_string(data: &[c_char]) -> String {
    let c_str = unsafe { CStr::from_ptr(data.as_ptr()) };
    c_str.to_str()
        .unwrap()
        .trim_matches(char::from(0))
        .to_string()
}

fn parse_kernel_version(v: u32) -> (u8, u8, u8) {
    let major = ((v >> 16) & 0xff) as u8;
    let minor = ((v >> 8) & 0xff) as u8;
    let patch = (v & 0xff) as u8;
    (major, minor, patch)
}


fn get_device_path_from_interface(interface: &Interface) -> PathBuf {
    lazy_static! {
        static ref DEVNAME_REGEX: Regex = Regex::new(r"(?m)^DEVNAME=(.+)$").unwrap();
    }

    let major = interface.major;
    let minor = interface.minor;

    let mut path = PathBuf::from(format!("/sys/dev/char/{}:{}/uevent", major, minor));
    let path_string = path.to_string_lossy().to_string();

    let ue = fs::read_to_string(path)
        .expect(&format!("Couldn't read file at {}", &path_string));

    let caps = DEVNAME_REGEX.captures(&ue).unwrap();

    let devname = &caps[1].to_string();

    let out = PathBuf::from(format!("/dev/{}", &devname));

    println!("Found device in {:#?}", &out);

    out
}
