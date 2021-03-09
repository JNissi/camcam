use std::process::Command;

fn main() {
    Command::new("glib-compile-resources")
        .arg("--target=src/camcam.gresource")
        .args(&["--sourcedir=src", "src/camcam.xml"])
        .status()
        .unwrap();

    // List all files that go into the resources file for rerun check.
    println!("cargo:rerun-if-changed=src/camcam.xml");
    println!("cargo:rerun-if-changed=src/camcam.glade");
}
