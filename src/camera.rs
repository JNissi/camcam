use crate::picture::Picture;
use v4l::prelude::*;
use v4l::context;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::format::{Format, Flags, fourcc::FourCC, field::FieldOrder, colorspace::Colorspace, quantization::Quantization, transfer::TransferFunction};

use relm::Sender;
use std::{fs, io, path::{Path, PathBuf}, sync::{Arc, Mutex, RwLock}, thread};
use media_device::MediaDevice;

mod convert;
mod media_ioctl;
mod media_device;
mod topology;
mod subdevice;
mod video_device;


const CAMERA_NAME: &str = "sun6i-csi";

#[derive(Clone, Copy)]
enum Sensor {
    Back,
    Front
}

pub enum CamMsg {
    Ready(Camera),
    Pic(Picture),
    Captured
}

pub struct Camera {
    main_device: Arc<RwLock<Device>>,
    should_preview: Arc<RwLock<bool>>,
    media_device: Arc<RwLock<MediaDevice>>,
    sender: Arc<Mutex<Sender<CamMsg>>>,
    sensor: Sensor,
    thread_handle: Option<thread::JoinHandle<()>>
}

impl Camera {
    // Highly Pinephone specific detection.
    pub fn detect(sender: Sender<CamMsg>) {
        if let Some(d) = context::enum_devices().iter().find(|d| {
            let name = d.name();
            name.is_some() && name.unwrap() == CAMERA_NAME
        }) {
            let media_device_path = guess_media_device_path(&d.path()).expect("No media device path found.");
            let mut media_device = MediaDevice::open(media_device_path).expect("Can't open media device");

            media_device.setup();

            // Device open will take ~10s if back camera is linked.
            let device = Device::new(d.index()).expect("Couldn't get camera device.");

            let sender = Arc::new(Mutex::new(sender));
            let sender_copy = sender.clone();

            let cam = Camera {
                main_device: Arc::new(RwLock::new(device)),
                should_preview: Arc::new(RwLock::new(false)),
                media_device: Arc::new(RwLock::new(media_device)),
                sender: sender,
                sensor: Sensor::Back,
                thread_handle: None
            };

            sender_copy.lock()
                .expect("Can't lock cam msg sender.")
                .send(CamMsg::Ready(cam))
                .expect("Can't send camera ready.");

        }
    }

    pub fn switch_sensor(&mut self) {
        println!("Camera switching sensor.");
        self.stop_preview();
        println!("Preview stopped.");
        self.sensor = match self.sensor {
            Sensor::Back => Sensor::Front,
            Sensor::Front => Sensor::Back
        };
        println!("Starting preview");
        self.start_preview();
        println!("Preview started");
    }

    pub fn stop_preview(&mut self) {
        {
            let mut sp = self.should_preview.write().unwrap();
            *sp = false;
        } // Drop write lock. Otherwise read can't happen in the preview loop.
        if let Some(handle) = self.thread_handle.take() {
            handle.join();
        }
    }

    pub fn start_preview(&mut self) {
        let dev = self.main_device.clone();
        let sender = self.sender.clone();
        let preview_lock = self.should_preview.clone();
        {
            let mut sp = preview_lock.write().unwrap();
            *sp = true;
        }
        let media_device = self.media_device.clone();
        let sensor = self.sensor;

        let (w, h, denominator) = match sensor {
            Sensor::Back => (1280, 720, 30),
            Sensor::Front => (1280, 960, 15)
        };


        let thread_handle = thread::spawn(move || {

            let md = media_device.write()
                .expect("Couldn't lock media device.");
            match sensor {
                Sensor::Back => {
                    md.unlink_front_camera();
                    md.link_back_camera();
                    md.set_back_format(w, h);
                    md.set_back_interval(1, denominator);
                },
                Sensor::Front => {
                    md.unlink_back_camera();
                    md.link_front_camera();
                    md.set_front_format(w, h);
                    md.set_front_interval(1, denominator);
                    md.hflip_front(true);
                }
            }

            let format = Format {
                width: w,
                height: h,
                fourcc: FourCC::new(b"BA81"),
                field_order: FieldOrder::Progressive,
                stride: w,
                size: w * h,
                flags: Flags::empty(),
                colorspace: Colorspace::RAW,
                quantization: Quantization::Default,
                transfer: TransferFunction::None
            };

            let num_bufs = 4;
            let mut dev = dev.write().unwrap();
            dev.set_format(&format).expect("Can't set video device buffer.");
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

            println!("w: {}, h: {}, s: {}", width, height, stride);


            // The driver seems to expect all buffers queued before start
            // Otherwise it just gets stuck at first stream.next()
            for i in 1..num_bufs as usize {
                stream.queue(i).expect("Can't queue buffer");
            }

            //stream.start();

            while *preview_lock.read().unwrap() == true {
                let (buf, _meta) = stream.next()
                    .expect("Failure when reading picture from MmapStream!");
                let buf_len = buf.len();
                if buf_len == 0 {
                    continue;
                }

                let data = debayer_superpixel(buf, width, height);

                let width = width / 2;
                let height = height / 2;
                let rowstride = width * 3;

                let data = glib::Bytes::from_owned(data);

                let data = Picture::new(
                    width as i32,
                    height as i32,
                    rowstride as i32,
                    data);

                sender.lock().unwrap().send(CamMsg::Pic(data)).expect("Can't send picture buffer.");

            }

            println!("Done with preview thread.");

        });

        self.thread_handle = Some(thread_handle);
    }

    pub fn capture(&self) {
        // Sleep a bit so as not to hang on device busy error.
        // TODO: Properly handle device busy and remove sleep.
        thread::sleep_ms(200);
        let dev = self.main_device.clone();
        let preview_lock = self.should_preview.clone();
        {
            let mut sp = preview_lock.write().unwrap();
            *sp = true;
        }

        let (w, h, denominator) = match self.sensor {
            Sensor::Back => (2592, 1944, 15),
            Sensor::Front => (1600, 1200, 15)
        };

        let media_device = self.media_device.clone();
        let md = media_device.write()
            .expect("Couldn't lock media device.");

        match self.sensor {
            Sensor::Back => {
                md.link_back_camera();
                md.set_back_interval(1, denominator);
                md.set_back_format(w, h);
            },
            Sensor::Front => {
                md.link_front_camera();
                md.set_front_interval(1, denominator);
                md.set_front_format(w, h);
            }
        }

        let format = Format {
            width: w,
            height: h,
            fourcc: FourCC::new(b"BA81"),
            field_order: FieldOrder::Progressive,
            stride: w,
            size: w * h,
            flags: Flags::empty(),
            colorspace: Colorspace::RAW,
            quantization: Quantization::Default,
            transfer: TransferFunction::None
        };

        let num_bufs = 4;
        let mut dev = dev.write().unwrap();
        dev.set_format(&format).expect("Can't set video device format.");
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
        let _stride = format.stride;

        for i in 1..num_bufs as usize {
            stream.queue(i).expect("Can't queue buffer");
        }

        let mut count = 0;

        loop {
            let (buf, _meta) = stream.next()
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
            self.sender.clone().lock().unwrap().send(CamMsg::Captured).expect("Can't send status.");
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

// < 30ms
fn debayer_superpixel(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    // B G
    // G R
    // Slice access is 10x slower than vec access
    let data = data.to_vec();
    let width = width as usize;
    let height = height as usize;
    let out_w = width / 2;
    let out_h = height / 2;
    let mut out = Vec::with_capacity(out_w * out_h * 3);
    let super_pix = [0, 1, width, width + 1];
    let len = data.len();

    for row in (0..len).step_by(width + width) {
        for col in (0..width).step_by(2) {
            let top_left = row + col;

            out.push(data[super_pix[3] + top_left]);
            /*let g = ((
                    data[super_pix[1] + top_left] as usize +
                    data[super_pix[2] + top_left] as usize
                ) >> 1) as u8;
            out.push(g);*/
            out.push(data[super_pix[1] + top_left]);
            out.push(data[super_pix[0] + top_left]);
      }
    }

    out
}

