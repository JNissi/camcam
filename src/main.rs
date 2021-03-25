use gdk_pixbuf::{Colorspace, Pixbuf, PixbufRotation};
use gtk::{prelude::{BuilderExtManual}, ApplicationWindow, Builder, Button, ButtonExt, Image, ImageExt, Inhibit, WidgetExt};
use relm::{connect, Channel, Relm, Update, Widget};
use relm_derive::Msg;
use std::thread;


mod camera;
use camera::{ Camera, CamMsg };

mod picture;
use picture::Picture;

mod sensor_proxy;
use sensor_proxy::SensorProxyProxy;

struct Model<'a> {
    _channel: Channel<CamMsg>,
    camera: Option<Camera>,
    sensor_proxy: SensorProxyProxy<'a>
}

use self::Msg::*;

#[derive(Msg)]
enum Msg {
    Cam(Camera),
    Pic(Picture),
    Shutter,
    PhotoDone,
    Unfocus,
    Focus,
    Quit,
    SwitchCamera
}

struct Widgets {
    window: ApplicationWindow,
    preview: Image
}

struct MainWin<'a> {
    model: Model<'a>,
    widgets: Widgets,
}

impl<'a> Update for MainWin<'a> {
    type Model = Model<'a>;
    type ModelParam = ();
    type Msg = Msg;

    fn model(relm: &Relm<Self>, _: ()) -> Model<'a> {
        let connection = zbus::Connection::new_system().expect("Can't connect to system dbus");
        let proxy = SensorProxyProxy::new(&connection).expect("Can't construct sensor proxy proxy.");
        let stream = relm.stream().clone();

        let (channel, sender) = Channel::new(move |msg| {
            match msg {
                CamMsg::Ready(cam) => stream.emit(Cam(cam)),
                CamMsg::Pic(pic) => stream.emit(Pic(pic)),
                CamMsg::Captured => stream.emit(PhotoDone)
            }
        });

        thread::spawn(move || {
            Camera::detect(sender);
        });

        Model {
            _channel: channel,
            camera: None,
            sensor_proxy: proxy
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => gtk::main_quit(),
            Cam(mut cam) => {
                cam.start_preview();
                self.model.camera = Some(cam)
            },
            Pic(pic) => {
                let pb = Pixbuf::from_bytes(
                    &pic.data(),
                    Colorspace::Rgb,
                    false,
                    8,
                    pic.width(),
                    pic.height(),
                    pic.rowstride()
                );

                let pb = pb.rotate_simple(PixbufRotation::Clockwise).unwrap();

                self.widgets.preview.set_from_pixbuf(Some(&pb));
                //self.widgets.window.show_all();
            },
            Shutter => {
                self.model.camera.as_mut().unwrap().stop_preview();
                let orientation = self.model.sensor_proxy.accelerometer_orientation();
                let orientation = match orientation {
                    Ok(o) => o,
                    Err(_) => "undefined".to_string()
                };
                self.model.camera.as_ref().unwrap().capture(orientation);
            },
            PhotoDone => {
                self.model.camera.as_mut().unwrap().start_preview();
            },
            Unfocus => {
                self.model.sensor_proxy.release_accelerometer();
                if let Some(cam) = self.model.camera.as_mut() {
                    cam.stop_preview();
                }
                println!("Should stop preview.");
            },
            Focus => {
                self.model.sensor_proxy.claim_accelerometer();
                if let Some(cam) = self.model.camera.as_mut() {
                    cam.start_preview();
                }
                println!("Should start preview.");
            },
            SwitchCamera => {
                if let Some(cam) = self.model.camera.as_mut() {
                    cam.switch_sensor();
                }
                println!("Switch camera.");
            }
        }
    }
}

impl Widget for MainWin<'_> {
    type Root = ApplicationWindow;

    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let bytes = glib::Bytes::from_static(include_bytes!("camcam.gresource"));
        let res = gio::Resource::from_data(&bytes).expect("Can't get resources from bytes.");
        gio::resources_register(&res);

        let builder = Builder::from_resource("/app/camcam.glade");

        let window: ApplicationWindow = builder
            .get_object("main_window")
            .expect("Can't get main window.");

        let preview: Image = builder
            .get_object("preview")
            .expect("Can't get preview image widget.");


        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );

        connect!(
            relm,
            window,
            connect_focus_in_event(_, _),
            return (Some(Msg::Focus), Inhibit(false))
        );

        connect!(
            relm,
            window,
            connect_focus_out_event(_, _),
            return (Some(Msg::Unfocus), Inhibit(false))
        );

        let shutter: Button = builder
            .get_object("shutter")
            .expect("Can't get shutter button.");

        connect!(
            relm,
            shutter,
            connect_clicked(_),
            Msg::Shutter
        );

        let camera_switch: Button = builder
            .get_object("camera_switch")
            .expect("Can't get camera switch button.");

        connect!(
            relm,
            camera_switch,
            connect_clicked(_),
            Msg::SwitchCamera
        );

        window.show_all();

        MainWin {
            model,
            widgets: Widgets {
                window,
                preview
            }
        }
    }
}

fn main() {
    MainWin::run(()).expect("Main win run failed!");
}
