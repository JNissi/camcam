use crate::camera::topology::*;

pub struct VideoDevice {
    // No need for handle, that's handled (pun intented) on Camera level.
    pub entity: Entity,
    pub interface: Interface,
    pub pad: Pad
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
