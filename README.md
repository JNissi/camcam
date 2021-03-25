# Camcam Camera for Pinephone Phosh

*(Try saying that quickly three times in a row)*

As for the name: All names are taken anyway, so why not keep it simple.

A proper phone application camera for the Pinephone. Written in Rust, using the Relm library for a bit more ergonomic GTK and v4l-rs for most of the camera handling. All the camera code is highly Pinephone specific. Do not expect this to work in any way on any non-Pinephone.

Developed on Manjaro Phosh. Might work on Mobian, depending on how badly outdated it gets...

## Requirements
 * libgexiv `sudo pacman -S libgexiv2`
   * Hopefully I can get rid of this at some point with another exif library

## Goals (short term)
 * ☐ Quick & dirty pictures on Pinephone
 * ☐ Quick & passable pictures on Pinephone
 * ☐ Quick & pretty pictures on Pinephone

## Goals (longer term)
 * ☐ Proper settings and adjustments
 * ☐ Extract the camera handling as a library with a sane api.

## Non-goals (as far as I can see)
 * Slow
 * Other devices
 * Other visual toolkits
 * Extra cameras
 * Video recording
 * Everything and the kitchen sink

## What to expect as of 25th March 2021 (I'll try to keep this up-to-date)
 * Both cameras work.
 * Might need to run Megapixels first to fix some kinks.
 * Photo quality is not much to write home about. A bit dark and there's some noise, but you can tell by looking at the photo what the subject is.
 * Only orientation exif data is saved, but at least the images are now correctly rotated in viewer.
 
![a tired cat under a plant](https://raw.githubusercontent.com/JNissi/camcam/main/example_photos/camcam-2021-03-09-07-02-01.jpg)

## Acknowledgements

A huge thanks goes to Martijn Braam for figuring out the Pinephone camera setup for their app Megapixels. Seriously, check it out if you need more configurability.
 
## License
MIT

