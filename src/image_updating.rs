#![allow(static_mut_refs)]
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
};
use gdk4::{MemoryTexture, glib::Bytes};
use gtk4::prelude::WidgetExt;
use image::{DynamicImage, GenericImage, Rgba};

use crate::{
    ACTIVE_LAYER_X, COLOR, COPY_TO_CLIPBOARD, IMG_HEIGHT, IMG_READ, IMG_WIDTH, LAYERS, Layer, NEEDS_FULL, PICTURE_WIDGET, QUEUE, SAVE_PATH, SIZE, WIDGET_SIZE,
    copy_to_clipboard::copy_to_clipbard,
};

pub fn update_image() {
    let lock = &*IMG_READ.lock().unwrap();
    let Some(main_image) = &lock else {
        panic!("uh");
    };

    let mut main_vvimg = main_image.clone();

    let width = main_vvimg.width();
    let height = main_vvimg.height();
    let mut last_iter = 0;
    loop {
        let amount_of_ms: i64 = 16 - last_iter;
        thread::sleep(Duration::from_millis(amount_of_ms.max(0) as u64));

        let time = Instant::now();
        let changed = draw(&mut main_vvimg);
        let needs_full = NEEDS_FULL.load(std::sync::atomic::Ordering::Relaxed);
        let copy = COPY_TO_CLIPBOARD.load(std::sync::atomic::Ordering::Relaxed);
        if !changed && needs_full && copy {
            last_iter = 0;
            continue;
        }

        if needs_full {
            NEEDS_FULL.store(false, std::sync::atomic::Ordering::Relaxed);
            let mut n_img = main_image.clone();
            let layers = LAYERS.lock().unwrap();

            for (layer, is_active) in layers.iter().rev() {
                if *is_active {
                    overlay(&mut n_img, layer);
                }
            }
            main_vvimg = n_img;
        }

        let bytes = main_vvimg.as_bytes();
        let bytes = Bytes::from(bytes);

        let texture = MemoryTexture::new(width as i32, height as i32, gdk4::MemoryFormat::R8g8b8a8, &bytes, (width * 4) as usize);

        unsafe {
            let Some(x) = &PICTURE_WIDGET else {
                panic!("");
            };
            let mut lock = WIDGET_SIZE.lock().unwrap();
            lock.0 = x.width();
            lock.1 = x.height();
            drop(lock);
            x.set_paintable(Some(&texture));
        }

        if copy {
            COPY_TO_CLIPBOARD.swap(false, std::sync::atomic::Ordering::Relaxed);
            let path = SAVE_PATH.get().cloned();
            copy_to_clipbard(&main_vvimg, path);
        }

        let elapsed = time.elapsed();
        last_iter = elapsed.as_millis() as i64;
        // println!("{:?}", elapsed)
    }
}

pub fn draw(img: &mut DynamicImage) -> bool {
    let mut lock = QUEUE.lock().unwrap();
    if lock.is_empty() {
        false
    } else {
        let elements = lock.len();
        let points: Vec<(i32, i32)> = lock.drain(0..elements).collect();
        drop(lock);
        let img_width = IMG_WIDTH.load(std::sync::atomic::Ordering::Relaxed);
        let img_height = IMG_HEIGHT.load(std::sync::atomic::Ordering::Relaxed);
        let size = SIZE.load(std::sync::atomic::Ordering::Relaxed);
        let lock = COLOR.lock().unwrap();
        let pixel = Rgba(*lock);
        drop(lock);
        let Some(arc) = ACTIVE_LAYER_X.read().unwrap().clone() else {
            return false;
        };
        let mut layer = arc.lock().unwrap();

        for pix in points {
            let circ = Circle::new(Point::new(pix.0, pix.1), size).into_styled(PrimitiveStyle::with_fill(Rgb888::WHITE));
            for n in circ.pixels() {
                if n.0.x >= img_width {
                    continue;
                }
                if n.0.y >= img_height {
                    continue;
                }
                layer.push((n.0.x, n.0.y, pixel));
                img.put_pixel(n.0.x as u32, n.0.y as u32, pixel);
            }
        }
        true
    }
}

fn overlay(bottom: &mut DynamicImage, top: &Arc<Layer>) {
    let lock = top.lock().unwrap();
    for n in lock.iter() {
        bottom.put_pixel(n.0 as u32, n.1 as u32, n.2);
    }
}
