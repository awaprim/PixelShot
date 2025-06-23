use std::{
    collections::VecDeque,
    io::{Cursor, Read},
    process::Stdio,
    sync::Mutex,
    thread,
    time::Duration,
};
mod copy_to_clipboard;
mod draw_line;
mod image_updating;
mod ui_interactions;

use ashpd::desktop::screenshot::Screenshot;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use gdk4::MemoryTexture;
use gtk::prelude::*;
use gtk4::{
    self as gtk, ApplicationWindow, Box, Button, EventControllerKey, EventControllerMotion,
    HeaderBar, Picture, glib::Bytes,
};
use image::{DynamicImage, ImageReader, Rgba};
use tokio::process::Command;

use crate::image_updating::{draw, update_image};
async fn take_screenshot_wayland_portal() -> String {
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
async fn take_screenshot() -> String {
    let os = std::env::consts::OS;
    match os {
        "linux" => {
            let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") else {
                panic!("unknown session type");
            };
            match session_type.as_str() {
                "x11" => {
                    panic!("unimplemented");
                }
                "wayland" => take_screenshot_wayland_portal().await,
                _ => {
                    panic!("unimplemented");
                }
            }
        }
        _ => {
            panic!("unimplemented");
        }
    }
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
    if cfg!(debug_assertions) {
        return handle_debug();
    }

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

pub static mut PICTURE_WIDGET: Option<Picture> = None;
static mut IMG_WIDTH: i32 = 0;
static mut IMG_HEIGHT: i32 = 0;
pub static IMG_READ: Mutex<Option<DynamicImage>> = Mutex::new(None);
pub static mut CHANGED: bool = false;
pub static mut QUEUE: VecDeque<(i32, i32)> = VecDeque::new();
static mut LAST_FRAME: (i32, i32) = (-1, -1);
pub static mut NEEDS_FULL: bool = false;
pub static mut COPY_TO_CLIPBOARD: bool = false;
pub static mut LAYERS: Vec<(Vec<(i32, i32, Rgba<u8>)>, bool)> = Vec::new();
pub static mut ACTIVE_LAYER: Option<*mut Vec<(i32, i32, Rgba<u8>)>> = None;
pub static mut V_IMG: Option<*mut DynamicImage> = None;
static mut WINDOW: Option<ApplicationWindow> = None;
pub static mut COLOR: [u8; 4] = [255, 0, 0, 255];
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
    let copy_to_clipboard_button = Button::builder().icon_name("edit-copy").build();

    menu_button.connect_clicked(ui_interactions::menu_button);
    copy_to_clipboard_button.connect_clicked(ui_interactions::copy_to_clipbard_button);
    let toolbar = HeaderBar::new();
    toolbar.pack_start(&menu_button);
    toolbar.pack_end(&copy_to_clipboard_button);

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

    key_controller.connect_key_pressed(ui_interactions::key_pressed);
    motion_controller.connect_motion(ui_interactions::mouse_move);

    img.add_controller(motion_controller);
    main_box.append(&aspect_frame);
    window.set_child(Some(&main_box));
    // window.set_child(Some(&aspect_frame));
    unsafe {
        PICTURE_WIDGET = Some(img);
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
