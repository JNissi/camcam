use chrono::prelude::*;
use image;
use lazy_static::lazy_static;
use std::{env, path::{Path, PathBuf}};

lazy_static! {
    static ref PICTURES_DIR: PathBuf = match dirs::picture_dir() {
        Some(dir) => dir,
        None => {
            let home = env::var("HOME").expect("Can't get $HOME. This seems bad.");
            let home = Path::new(&home);
            println!("Couldn't find configured pictures dir (XDG). Defaulting to $HOME/Pictures");
            home.join("Pictures")
        }
    };
}

pub fn save(data: Vec<u8>, width: usize, height: usize) {
    let (r, g, b) = separate_colors(&data, width, height);
    let r = smudge_red(&r, width, height);
    let g = smudge_green(&g, width, height);
    let b = smudge_blue(&b, width, height);
    let data = combine_rgb(&r, &g, &b);

    let now = Local::now();
    let time_part = now.format("%Y-%m-%d-%H-%M-%S");
    let mut pic_path = PICTURES_DIR.clone();
    pic_path.push(format!("camcam-{}.jpg", time_part));
    image::save_buffer(&pic_path, &data, width as u32, height as u32, image::ColorType::Rgb8);
}

fn combine_rgb(r: &[u8], g: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = vec![0; r.len() * 3];
    assert_eq!(r.len(), g.len());
    assert_eq!(g.len(), b.len());

    for i in 0..r.len() {
        let offset = i * 3;
        out[offset] = r[i];
        out[offset + 1] = g[i];
        out[offset + 2] = b[i];
    }

    out
}

fn smudge_red(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0; width * height];

    for row in (1..height).step_by(2) {
        for col in 0..width {
            let pos = row * width + col;
            if col % 2 == 1 {
                out[pos] = data[pos];
            } else {
                let mut count = 1;
                let mut value = data[pos + 1] as usize;

                if col > 0 {
                    count += 1;
                    value += data[pos - 1] as usize;
                }

                out[pos] = (value / count) as u8;
            }
        }
    }

    for row in (0..height).step_by(2) {
        for col in 0..width {
            let pos = row * width + col;
            let mut count = 1;
            let mut value = out[pos + width] as usize;
            if row > 0 {
                count += 1;
                value += out[pos - width] as usize;
            }
            out[pos] = (value / count) as u8;
        }
    }

    out
}

fn smudge_blue(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0; width * height];

    for row in (0..height).step_by(2) {
        for col in 0..width {
            let pos = row * width + col;
            if col % 2 == 0 {
                out[pos] = data[pos];
            } else {
                let mut count = 1;
                let mut value = data[pos - 1] as usize;

                if col < width - 1 {
                    count += 1;
                    value += data[pos + 1] as usize;
                }

                out[pos] = (value / count) as u8;
            }
        }
    }

    for row in (1..height).step_by(2) {
        for col in 0..width {
            let pos = row * width + col;
            let mut count = 1;
            let mut value = out[pos - width] as usize;
            if row < height - 1 {
                count += 1;
                value += out[pos + width] as usize;
            }
            out[pos] = (value / count) as u8;
        }
    }

    out
}

fn smudge_green(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0; width * height];

    for row in 0..height {
        for col in 0..width {
            let rem =  row % 2;

            if col % 2 == rem {
                let mut count = 0;
                let mut value = 0usize;
                if row > 0 {
                    count += 1;
                    value += data[(row - 1) * width + col] as usize;
                }

                if col < width - 1 {
                    count += 1;
                    value += data[row * width + col + 1] as usize;
                }

                if row < height - 1 {
                    count += 1;
                    value += data[(row + 1) * width + col] as usize;
                }

                if col > 0 {
                    count += 1;
                    value += data[row * width + col - 1] as usize;
                }

                out[row * width + col] = (value / count) as u8;
            } else {
                out[row * width + col] = data[row * width + col];
            }
        }
    }

    out
}

fn separate_colors(data: &[u8], width: usize, height: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut r = vec![0; width * height];
    let mut g = vec![0; width * height];
    let mut b = vec![0; width * height];

    for row in 0..height {
        if row % 2 == 0 {
            for col in 0..width {
                if col % 2 == 0 {
                    b[row * width + col] = data[row * width + col];
                } else {
                    g[row * width + col] = data[row * width + col];
                }
            }
        } else {
            for col in 0..width {
                if col % 2 == 0 {
                    g[row * width + col] = data[row * width + col];
                } else {
                    r[row * width + col] = data[row * width + col];
                }
            }
        }
    }


    (r, g, b)
}

