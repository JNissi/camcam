use linux_media::*;
use std::{
    ffi::CStr,
    os::raw::c_char,
};

#[derive(Debug)]
pub struct Topology {
    pub version: u64,
    pub entities: Vec<Entity>,
    pub interfaces: Vec<Interface>,
    pub pads: Vec<Pad>,
    pub links: Vec<Link>,
}

impl Topology {
    pub fn from(topology: media_v2_topology, entities: &[media_v2_entity], interfaces: &[media_v2_interface], pads: &[media_v2_pad], links: &[media_v2_link]) -> Topology {
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
    pub id: u32,
    pub name: String,
    pub function: u32,
    pub flags: u32,
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
    pub id: u32,
    pub interface_type: u32,
    pub flags: u32,
    pub major: u32,
    pub minor: u32
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
    pub id: u32,
    pub entity_id: u32,
    pub flags: u32,
    pub index: u32,
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
    pub id: u32,
    pub source_id: u32,
    pub sink_id: u32,
    pub flags: u32,
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

pub fn c_char_array_to_string(data: &[c_char]) -> String {
    let c_str = unsafe { CStr::from_ptr(data.as_ptr()) };
    c_str.to_str()
        .unwrap()
        .trim_matches(char::from(0))
        .to_string()
}

pub fn parse_kernel_version(v: u32) -> (u8, u8, u8) {
    let major = ((v >> 16) & 0xff) as u8;
    let minor = ((v >> 8) & 0xff) as u8;
    let patch = (v & 0xff) as u8;
    (major, minor, patch)
}

