use crate::picture::Picture;
use futures::StreamExt;
use v4l::prelude::*;
use v4l::context;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::io::traits::Stream;

use dirs;

use relm::Sender;
use std::{fs, fs::File, io, io::prelude::*, path::{Path, PathBuf}, sync::{Arc, Mutex, RwLock}, thread};

use media_device::MediaDevice;

mod convert;
mod media_ioctl;
mod media_device;


const CAMERA_NAME: &str = "sun6i-csi";

pub struct Camera {
    main_device: Arc<RwLock<Device>>,
    should_preview: Arc<RwLock<bool>>,
    media_device: MediaDevice,
    sender: Arc<Mutex<Sender<Picture>>>,
    status_sender: Arc<Mutex<Sender<()>>>,
}

impl Camera {
    // Highly Pinephone specific detection.
    pub fn detect(sender: Sender<Picture>, status_sender: Sender<()>) -> Option<Camera> {
        if let Some(d) = context::enum_devices().iter().find(|d| {
            let name = d.name();
            name.is_some() && name.unwrap() == CAMERA_NAME
        }) {
            let device = Device::with_path(&d.path()).expect("Couldn't get camera device.");
            let media_device_path = guess_media_device_path(&d.path()).expect("No media device path found.");
            let mut media_device = MediaDevice::open(media_device_path).expect("Can't open media device");
            media_device.setup();

            return Some(Camera {
                main_device: Arc::new(RwLock::new(device)),
                should_preview: Arc::new(RwLock::new(false)),
                media_device: media_device,
                sender: Arc::new(Mutex::new(sender)),
                status_sender: Arc::new(Mutex::new(status_sender))
            })
        }
        None
    }

    pub fn set_sender(&mut self, sender: Sender<Picture>) {
        self.sender = Arc::new(Mutex::new(sender));
    }

    pub fn stop_preview(&self) {
        let mut sp = self.should_preview.write().unwrap();
        *sp = false;
    }

    pub fn start_preview(&self) {
        let dev = self.main_device.clone();
        let sender = self.sender.clone();
        let preview_lock = self.should_preview.clone();
        {
            let mut sp = preview_lock.write().unwrap();
            *sp = true;
        }
        self.media_device.link_back_camera();

        thread::spawn(move || {
            let num_bufs = 4;
            let mut dev = dev.write().unwrap();
            let params = dev.params().expect("Couldn't get device params.");
            println!("Device params: {:#?}", params);
            let format = dev.format().expect("Couldn't get device format.");
            println!("Device format: {:#?}", format);
            let mut stream = MmapStream::with_buffers(&mut *dev, Type::VideoCapture, num_bufs)
                .expect("Failed to create MmapStream!");

            // 1280 x 960 BA81
            // BG
            // GR
            // 640x480 RGB

//            let format = dev.format().expect("Can't get current format!");
            let width = format.width;
            let height = format.height;
            let stride = format.stride;

            println!("w: {}, h: {}, s: {}", width, height, stride);

            for i in 1..num_bufs as usize {
                stream.queue(i);
            }


            //stream.start();

            while *preview_lock.read().unwrap() == true {
                let (buf, meta) = stream.next()
                    .expect("Failure when reading picture from MmapStream!");
                let buf_len = buf.len();
                if buf_len == 0 {
                    continue;
                }
                assert!(buf_len % 4 == 0);


                let data = debayer_superpixel(&buf, width, height);

                let width = width / 2;
                let height = height / 2;
                let rowstride = width * 3;

                let data = glib::Bytes::from_owned(data);

                let data = Picture::new(
                    width as i32,
                    height as i32,
                    rowstride as i32,
                    data);

                sender.lock().unwrap().send(data).expect("Can't send picture buffer.");

            }

        });
    }

    pub fn capture(&self) {
        thread::sleep_ms(200);

        let dev = self.main_device.clone();
        let sender = self.sender.clone();
        let preview_lock = self.should_preview.clone();
        {
            let mut sp = preview_lock.write().unwrap();
            *sp = true;
        }
        self.media_device.link_back_camera();

        let num_bufs = 4;
        let mut dev = dev.write().unwrap();
        let params = dev.params().expect("Couldn't get device params.");
        println!("Device params: {:#?}", params);
        let format = dev.format().expect("Couldn't get device format.");
        println!("Device format: {:#?}", format);
        let mut stream = MmapStream::with_buffers(&mut *dev, Type::VideoCapture, num_bufs)
            .expect("Failed to create MmapStream!");

        // 1280 x 960 BA81
        // BG
        // GR
        // 640x480 RGB

        let width = format.width;
        let height = format.height;
        let stride = format.stride;

        for i in 1..num_bufs as usize {
            stream.queue(i);
        }

        let mut count = 0;

        loop {
            let (buf, meta) = stream.next()
                .expect("Failure when reading picture from MmapStream!");
            let buf_len = buf.len();
            assert!(buf_len % 4 == 0);

            if count < 6 {
                count += 1;
                continue;
            }


            let buf = buf.to_vec();
            thread::spawn(move || {
                convert::save(buf, width as usize, height as usize);
            });
            self.status_sender.clone().lock().unwrap().send(()).expect("Can't send status.");
            break;
        }
    }
}

fn guess_media_device_path(camera_path: &Path) -> io::Result<PathBuf> {
    let device_file = camera_path.file_name().unwrap();
    let mut pb = PathBuf::from("/sys/class/video4linux");
    pb.push(&device_file);
    pb.push("device");
    let path = pb.as_path();

    let media_path = fs::read_dir(path)?
        .filter(|e| e.is_ok())
        .map(|e| e.unwrap().path())
        .filter(|e| e.is_dir())
        .map(|e| {
            let file_name = e.file_name().unwrap();
            file_name.to_string_lossy().to_string()
        })
        .find(|e| &e[0..5] == "media")
        .expect("Failed to find media device name for video device");

    let mut pb = PathBuf::from("/dev");
    pb.push(&*media_path);
    Ok(pb)
}

fn debayer_superpixel(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let out_w = width / 2;
    let out_h = height / 2;
    let mut out = vec![0; out_w * out_h * 3];

    for (out_row, row) in (0..height).step_by(2).enumerate() {
        let out_offset = out_row * out_w * 3;
        for (out_col, col) in (0..width).step_by(2).enumerate() {
            let out_offset = out_offset + out_col * 3;
            out[out_offset] = data[(row + 1) * width + col + 1];
            let mut g: u16 = data[row * width + col + 1] as u16;
            g += data[(row + 1) * width + col] as u16;
            let g = (g / 2) as u8;
            out[out_offset + 1] = g;
            out[out_offset + 2] = data[row * width + col];
        }
    }

    out
}
