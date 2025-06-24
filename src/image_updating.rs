use std::{
    thread,
    time::{Duration, Instant},
};

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
};
use gdk4::{MemoryTexture, glib::Bytes};
use image::{DynamicImage, GenericImage, Rgba};

use crate::{
    ACTIVE_LAYER, CHANGED, COLOR, COPY_TO_CLIPBOARD, IMG_HEIGHT, IMG_READ, IMG_WIDTH, LAYERS,
    NEEDS_FULL, PICTURE_WIDGET, QUEUE, SIZE, V_IMG, copy_to_clipboard::copy_to_clipbard,
};

pub fn update_image() {
    let lock = &*IMG_READ.lock().unwrap();
    let Some(main_image) = &lock else {
        panic!("uh");
    };

    let mut main_vvimg = main_image.clone();
    let m = &raw mut main_vvimg;
    unsafe {
        V_IMG = Some(m);
    }
    let width = main_vvimg.width();
    let height = main_vvimg.height();
    let mut last_iter = 0;
    loop {
        let amount_of_ms: i64 = 16 - last_iter;
        thread::sleep(Duration::from_millis(amount_of_ms.max(0) as u64));

        let time = Instant::now();
        unsafe {
            if !CHANGED {
                last_iter = 0;
                continue;
            }
            CHANGED = false;
        };

        unsafe {
            if NEEDS_FULL {
                NEEDS_FULL = false;
                let mut n_img = main_image.clone();
                let layers = &*&raw const LAYERS;

                for (layer, is_active) in layers.iter().rev() {
                    if *is_active {
                        overlay(&mut n_img, layer);
                    }
                }
                main_vvimg = n_img;
            }
        }

        let bytes = main_vvimg.as_bytes();
        let bytes = Bytes::from(bytes);

        let texture = MemoryTexture::new(
            width as i32,
            height as i32,
            gdk4::MemoryFormat::R8g8b8a8,
            &bytes,
            (width * 4) as usize,
        );

        unsafe {
            let pic = &raw const PICTURE_WIDGET;
            let Some(x) = &*pic else {
                panic!("");
            };
            x.set_paintable(Some(&texture));
        }

        unsafe {
            if COPY_TO_CLIPBOARD {
                COPY_TO_CLIPBOARD = false;
                copy_to_clipbard(&main_vvimg);
            }
        }

        let elapsed = time.elapsed();
        last_iter = elapsed.as_millis() as i64;
        // println!("{:?}", elapsed)
    }
}

pub fn draw() {
    let img_pointer = unsafe {
        loop {
            if let Some(ptr) = V_IMG {
                break ptr;
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    };
    let img = unsafe { &mut *img_pointer };
    let arr = unsafe { &mut *&raw mut QUEUE };

    loop {
        if arr.len() == 0 {
            thread::sleep(Duration::from_millis(50));
        } else {
            let layer = unsafe {
                let Some(ptr) = ACTIVE_LAYER else {
                    panic!("DRAW");
                };
                &mut *ptr
            };
            let pix = arr.pop_front().unwrap();
            let pixel = unsafe { Rgba(COLOR) };

            let circ = Circle::new(Point::new(pix.0, pix.1), unsafe { SIZE })
                .into_styled(PrimitiveStyle::with_fill(Rgb888::WHITE));
            for n in circ.pixels() {
                unsafe {
                    if n.0.x >= IMG_WIDTH {
                        continue;
                    }
                    if n.0.y >= IMG_HEIGHT {
                        continue;
                    }
                }
                layer.push((n.0.x, n.0.y, pixel));
                img.put_pixel(n.0.x as u32, n.0.y as u32, pixel);
            }
            unsafe {
                CHANGED = true;
            }
        }
    }
}

fn overlay(bottom: &mut DynamicImage, top: &Vec<(i32, i32, Rgba<u8>)>) {
    for n in top {
        bottom.put_pixel(n.0 as u32, n.1 as u32, n.2);
    }
}
