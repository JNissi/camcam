use glib::Bytes;

#[derive(Debug)]
pub struct Picture {
    width: i32,
    height: i32,
    rowstride: i32,
    data: Bytes
}

impl Picture {
    pub fn new(width: i32, height: i32, rowstride: i32, data: Bytes) -> Picture {
        Picture {
            width,
            height,
            rowstride,
            data
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn rowstride(&self) -> i32 {
        self.rowstride
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}
