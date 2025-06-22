use std::{
    collections::VecDeque,
    io::{Cursor, Read},
    process::{self, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use ashpd::desktop::screenshot::Screenshot;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
};
use gdk4::{Display, MemoryTexture};
use gtk::{glib, prelude::*};
use gtk4::{
    self as gtk, ApplicationWindow, Box, Button, ColorChooserDialog, ColorChooserWidget,
    EventControllerKey, EventControllerMotion, HeaderBar, Label, Picture,
    builders::ColorChooserWidgetBuilder,
    gdk::{Key, ModifierType},
    glib::Bytes,
};
use image::{
    DynamicImage, GenericImage, ImageBuffer, ImageReader, Pixel, Rgba, RgbaImage, imageops,
};
use tokio::process::Command;
async fn take_screenshot() -> String {
    if let Ok(hypr_env) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        if hypr_env.len() != 0 {
            let mut picker = Command::new("hyprpicker")
                .arg("-r")
                .arg("-z")
                .spawn()
                .unwrap();
            tokio::time::sleep(Duration::from_millis(250)).await;
            let response = Screenshot::request()
                .interactive(true)
                .modal(true)
                .send()
                .await
                .unwrap()
                .response()
                .unwrap();
            let _ = picker.kill().await;
            let uri = response.uri();
            return uri.path().to_string();
        }
    };
    let response = Screenshot::request()
        .interactive(true)
        .modal(true)
        .send()
        .await
        .unwrap()
        .response()
        .unwrap();
    let uri = response.uri();
    return uri.path().to_string();
}

fn handle_debug() {
    let mut file = std::fs::File::open("/home/idot/Downloads/n.png").unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    let mut lock = RAW_IMAGE.lock().unwrap();
    *lock = Some(bytes);
    drop(lock);
    let application = gtk::Application::builder()
        .application_id("me.awaprim.PixelShot")
        .build();
    application.connect_activate(build_ui);
    let args: Vec<&str> = Vec::new();
    application.run_with_args(&args);
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    #[cfg(debug_assertions)]
    return handle_debug();

    let args = std::env::args().skip(1);
    let img_path = take_screenshot().await;
    let mut edit = false;
    for n in args {
        match n.as_str() {
            "--editor" => {
                edit = true;
            }
            _ => {}
        }
    }
    if !edit {
        copy_no_ui(img_path).await;
    } else {
        let mut file = std::fs::File::open(img_path).unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        file.read_to_end(&mut bytes).unwrap();

        let mut lock = RAW_IMAGE.lock().unwrap();
        *lock = Some(bytes);
        drop(lock);

        let application = gtk::Application::builder()
            .application_id("com.screenshotting-tool.fisch")
            .build();
        application.connect_activate(build_ui);
        let args: Vec<&str> = Vec::new();
        application.run_with_args(&args);
    }
}
static RAW_IMAGE: Mutex<Option<Vec<u8>>> = Mutex::new(None);

async fn copy_no_ui(path: String) {
    let file = std::fs::File::open(path).unwrap();
    Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::from(file))
        .spawn()
        .unwrap();
}

static mut TEST: Option<Picture> = None;
static mut IMG_WIDTH: i32 = 0;
static mut IMG_HEIGHT: i32 = 0;
static IMG_READ: Mutex<Option<DynamicImage>> = Mutex::new(None);
static mut CHANGED: bool = false;
static mut QUEUE: VecDeque<(i32, i32)> = VecDeque::new();
static mut LAST_FRAME: (i32, i32) = (-1, -1);
static mut NEEDS_FULL: bool = false;
static mut COPY_TO_CLIPBOARD: bool = false;
static mut LAYERS: Vec<(Vec<(i32, i32, Rgba<u8>)>, bool)> = Vec::new();
static mut ACTIVE_LAYER: Option<*mut Vec<(i32, i32, Rgba<u8>)>> = None;
// TODO: potnetially deal with this mess

fn add_layer() {
    unsafe {
        let array = Vec::new();
        let layers = &mut *&raw mut LAYERS;
        layers.push((array, true));
        let Some(vec) = layers.last_mut() else {
            panic!("how??")
        };
        let vec_ptr = &raw mut vec.0;
        ACTIVE_LAYER = Some(vec_ptr);
    }
}

fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32) {
    if (y1 - y0).abs() < (x1 - x0).abs() {
        if x0 > x1 {
            draw_line_low(x1, y1, x0, y0);
        } else {
            draw_line_low(x0, y0, x1, y1);
        }
    } else {
        if y0 > y1 {
            draw_line_high(x1, y1, x0, y0);
        } else {
            draw_line_high(x0, y0, x1, y1);
        }
    }
    unsafe {
        (&mut *&raw mut QUEUE).push_back((x0, y0));
        (&mut *&raw mut QUEUE).push_back((x1, y1));
    }
}

fn draw_line_high(x0: i32, y0: i32, x1: i32, y1: i32) {
    let mut dx = x1 - x0;
    let dy = y1 - y0;
    let mut xi = 1;
    if dx < 0 {
        xi = -1;
        dx = -dx;
    }
    let mut d = (2 * dx) - dy;
    let mut x = x0;
    for y in y0..y1 {
        unsafe {
            (&mut *&raw mut QUEUE).push_back((x, y));
        }
        if d > 0 {
            x = x + xi;
            d = d + (2 * (dx - dy));
        } else {
            d = d + 2 * dx;
        }
    }
}
fn draw_line_low(x0: i32, y0: i32, x1: i32, y1: i32) {
    let dx = x1 - x0;
    let mut dy = y1 - y0;
    let mut yi = 1;
    if dy < 0 {
        yi = -1;
        dy = -dy;
    }
    let mut d = (2 * dy) - dx;
    let mut y = y0;
    for x in x0..x1 {
        unsafe {
            (&mut *&raw mut QUEUE).push_back((x, y));
        }
        if d > 0 {
            y = y + yi;
            d = d + (2 * (dy - dx));
        } else {
            d = d + 2 * dy;
        }
    }
}
static mut COLOR: [u8; 4] = [255, 0, 0, 255];
static mut WINDOW: Option<ApplicationWindow> = None;
fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_default_size(600, 400);
    let mut lock = RAW_IMAGE.lock().unwrap();
    let file = lock.take().unwrap();
    drop(lock);
    let vimg = ImageReader::new(Cursor::new(file))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    let vimg = vimg.into_rgba8();
    let vimg = DynamicImage::ImageRgba8(vimg);

    let menu_button = Button::builder().icon_name("open-menu-symbolic").build();

    menu_button.connect_clicked(|_| {
        // TODO: open setting menu with more options instead :3
        let window = unsafe {
            let Some(window) = &*&raw const WINDOW else {
                panic!("how");
            };
            window
        };

        let picker = ColorChooserDialog::new(Some("pick color"), Some(window));
        picker.run_async(|picker, resp| {
            let color = picker.rgba();
            unsafe {
                COLOR = [
                    (color.red() * 255.0) as u8,
                    (color.green() * 255.0) as u8,
                    (color.blue() * 255.0) as u8,
                    (color.alpha() * 255.0) as u8,
                ];
            };
            picker.close();
        });
    });
    let toolbar = HeaderBar::new();
    toolbar.pack_start(&menu_button);

    window.set_titlebar(Some(&toolbar));

    let main_box = Box::new(gtk4::Orientation::Horizontal, 10);
    // let wbox = Box::new(gtk4::Orientation::Horizontal, 5);
    // main_box.append(&wbox);

    let bytes = vimg.as_bytes();
    let height = vimg.height();
    let width = vimg.width();
    println!("{width} {height}");
    let bytes = Bytes::from(&bytes);
    let texture = MemoryTexture::new(
        width as i32,
        height as i32,
        gdk4::MemoryFormat::R8g8b8a8,
        &bytes,
        (width * 4) as usize,
    );

    let mut lock = IMG_READ.lock().unwrap();
    *lock = Some(vimg);
    drop(lock);

    let ratio = width as f32 / height as f32;
    unsafe {
        IMG_WIDTH = (width - 1) as i32;
        IMG_HEIGHT = (height - 1) as i32;
    }
    let img = Picture::for_paintable(&texture);
    img.set_content_fit(gtk4::ContentFit::Fill);
    let aspect_frame = gtk::AspectFrame::new(0.5, 0.5, ratio, false);
    aspect_frame.set_child(Some(&img));
    let motion_controller = EventControllerMotion::new();
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(|_, key, _, modifier| {
        if (key.eq(&Key::c) || key.eq(&Key::C)) && modifier.eq(&ModifierType::CONTROL_MASK) {
            unsafe {
                CHANGED = true;
                COPY_TO_CLIPBOARD = true;
            }
        }
        if (key.eq(&Key::z) || key.eq(&Key::Z))
            && modifier.eq(&(ModifierType::SHIFT_MASK | ModifierType::CONTROL_MASK))
        {
            unsafe {
                let overlays = &mut *&raw mut LAYERS;
                for (_, active) in overlays.iter_mut() {
                    if !*active {
                        *active = true;
                        break;
                    }
                }

                CHANGED = true;
                NEEDS_FULL = true;
            }
        }
        if (key.eq(&Key::z) || key.eq(&Key::Z)) && modifier.eq(&ModifierType::CONTROL_MASK) {
            unsafe {
                let overlays = &mut *&raw mut LAYERS;
                for (_, active) in overlays.iter_mut().rev() {
                    if *active {
                        *active = false;
                        break;
                    }
                }

                CHANGED = true;
                NEEDS_FULL = true;
            }
        }
        glib::Propagation::Proceed
    });

    motion_controller.connect_motion(|controller, x, y| {
        if controller
            .current_event_state()
            .eq(&ModifierType::BUTTON1_MASK)
        {
            let (w, h) = unsafe {
                let pic = &raw const TEST;
                let Some(x) = &*pic else {
                    panic!("");
                };
                (x.width(), x.height())
            };
            let (real_height, real_width) = unsafe {
                (
                    (((y as f64 / h as f64).min(1.0) * (IMG_HEIGHT) as f64) as i32).max(0),
                    (((x as f64 / w as f64).min(1.0) * (IMG_WIDTH) as f64) as i32).max(0),
                )
            };
            unsafe {
                if LAST_FRAME.0 != -1 {
                    draw_line(LAST_FRAME.0, LAST_FRAME.1, real_width, real_height);
                } else {
                    add_layer();
                    (&mut *&raw mut QUEUE).push_back((real_width, real_height));
                }
                LAST_FRAME = (real_width, real_height);
            }
        } else {
            unsafe {
                LAST_FRAME = (-1, -1);
            }
        };
    });
    img.add_controller(motion_controller);

    main_box.append(&aspect_frame);
    window.set_child(Some(&main_box));
    // window.set_child(Some(&aspect_frame));
    unsafe {
        TEST = Some(img);
    }
    thread::spawn(|| {
        update_image();
    });
    thread::spawn(|| {
        draw();
    });

    window.add_controller(key_controller);

    window.present();
    unsafe {
        WINDOW = Some(window);
    }
}

fn draw() {
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
            let circ = Circle::new(Point::new(pix.0, pix.1), 3)
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
static mut V_IMG: Option<*mut DynamicImage> = None;

fn update_image() {
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
            let pic = &raw const TEST;
            let Some(x) = &*pic else {
                panic!("");
            };
            x.set_paintable(Some(&texture));
        }

        unsafe {
            if COPY_TO_CLIPBOARD {
                COPY_TO_CLIPBOARD = false;
                let dis = Display::default().unwrap();
                let clipboard = dis.clipboard();
                clipboard.set_texture(&texture);
                continue;
            }
        }

        let elapsed = time.elapsed();
        last_iter = elapsed.as_millis() as i64;
        // println!("FINAL: {:?}", elapsed);
    }
}
