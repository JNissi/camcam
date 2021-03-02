use gdk_pixbuf::{Colorspace, Pixbuf, PixbufRotation};
use gtk::{prelude::BuilderExtManual, ApplicationWindow, Builder, Button, ButtonExt, Image, ImageExt, Inhibit, WidgetExt};
use relm::{connect, Channel, Relm, Sender, Update, Widget};
use relm_derive::Msg;
use std::{path::PathBuf, thread};


mod camera;
use camera::Camera;

mod picture;
use picture::Picture;

struct Model {
    channel: Channel<Picture>,
    status_channel: Channel<()>,
    camera: Camera
}

use self::Msg::*;

#[derive(Msg)]
enum Msg {
    Pic(Picture),
    Shutter,
    PhotoDone,
    Quit,
}

struct Widgets {
    window: ApplicationWindow,
    preview: Image
}

struct MainWin {
    model: Model,
    widgets: Widgets,
}

impl Update for MainWin {
    type Model = Model;
    type ModelParam = ();
    type Msg = Msg;

    fn model(relm: &Relm<Self>, _: ()) -> Model {
        let stream = relm.stream().clone();

        let (channel, sender) = Channel::new(move |pic| {
            stream.emit(Pic(pic));
        });

        let stream = relm.stream().clone();

        let (status_channel, status_sender) = Channel::new(move |_| {
            stream.emit(PhotoDone);
        });

        let mut camera = Camera::detect(sender, status_sender).expect("Couldn't get camera.");

        camera.start_preview();

        Model {
            channel,
            status_channel,
            camera
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Quit => gtk::main_quit(),
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
                self.model.camera.stop_preview();
                self.model.camera.capture();
            },
            PhotoDone => {
                self.model.camera.start_preview();
            }
        }
    }
}

impl Widget for MainWin {
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

        let shutter: Button = builder
            .get_object("shutter")
            .expect("Can't get shutter button.");

        window.show_all();

        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(Msg::Quit), Inhibit(false))
        );

        connect!(
            relm,
            shutter,
            connect_clicked(_),
            Msg::Shutter
        );

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
