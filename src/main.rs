use std::{
    collections::VecDeque,
    io::{Cursor, Read},
    path::PathBuf,
    process,
    str::FromStr,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicI32, AtomicU32},
    },
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
    self as gtk, Adjustment, ApplicationWindow, Box, Button, EventControllerKey,
    EventControllerMotion, HeaderBar, Picture, SpinButton, glib::Bytes,
};
use image::{DynamicImage, ImageReader, Rgba};
use once_cell::sync::OnceCell;
use tokio::process::Command;

use crate::{
    image_updating::update_image,
    ui_interactions::{changed_size, color_picker},
};

async fn take_screenshot_wayland_slurp_grim() -> String {
    if let Ok(hypr_env) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        && !hypr_env.is_empty()
    {
        let mut picker = Command::new("hyprpicker")
            .arg("-r")
            .arg("-z")
            .spawn()
            .unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;

        let region = Command::new("slurp")
            .arg("-d")
            .output()
            .await
            .unwrap()
            .stdout;
        let region = String::from_utf8(region).unwrap().replace("\n", "");
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = Command::new("grim")
            .arg("-g")
            .arg(region)
            .arg("/tmp/screenshot.png")
            .output()
            .await
            .unwrap();

        let _ = Command::new("pkill")
            .args(["-15", "hyprpicker"])
            .output()
            .await;
        let _ = picker.kill().await;
        return "/tmp/screenshot.png".to_string();
    };
    let region = Command::new("slurp")
        .arg("-d")
        .output()
        .await
        .unwrap()
        .stdout;
    let region = String::from_utf8(region).unwrap().replace("\n", "");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = Command::new("grim")
        .arg("-g")
        .arg(region)
        .arg("/tmp/screenshot.png")
        .output()
        .await
        .unwrap();

    "/tmp/screenshot.png".to_string()
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
                "wayland" => take_screenshot_wayland_slurp_grim().await,
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

fn handle_args(mut args: impl Iterator<Item = String>) -> (bool, Option<PathBuf>) {
    let mut edit = false;
    let mut file_to_edit = None;
    while let Some(n) = args.next() {
        match n.as_str() {
            "--editor" => {
                edit = true;
            }
            "--save" => {
                let Some(path) = args.next() else {
                    panic!("Invalid path")
                };
                let Ok(path) = PathBuf::from_str(&path);
                let _ = SAVE_PATH.set(path);
            }
            "--help" => {
                println!("{}", include_str!("./help.txt"));
                process::exit(0);
            }
            "--edit" => {
                let Some(path) = args.next() else {
                    panic!("Invalid path")
                };
                let Ok(path) = PathBuf::from_str(&path);
                file_to_edit = Some(path);
            }

            _ => {}
        }
    }
    (edit, file_to_edit)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = std::env::args().skip(1);
    let (edit, file_to_edit) = handle_args(args);

    if file_to_edit.is_some() {
        let mut file = std::fs::File::open(file_to_edit.unwrap()).unwrap();
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

        process::exit(0);
    }

    let img_path = take_screenshot().await;

    if !edit {
        let img = ImageReader::open(img_path)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let path = SAVE_PATH.get().cloned();

        copy_to_clipboard::copy_to_clipbard(&img, path);
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

pub static mut PICTURE_WIDGET: Option<Picture> = None;

static WIDGET_SIZE: Mutex<(i32, i32)> = Mutex::new((0, 0));

static IMG_WIDTH: AtomicI32 = AtomicI32::new(0);
static IMG_HEIGHT: AtomicI32 = AtomicI32::new(0);
pub static IMG_READ: Mutex<Option<DynamicImage>> = Mutex::new(None);
pub static QUEUE: Mutex<VecDeque<(i32, i32)>> = Mutex::new(VecDeque::new());
pub static SIZE: AtomicU32 = AtomicU32::new(3);
pub static NEEDS_FULL: AtomicBool = AtomicBool::new(false);
pub static COPY_TO_CLIPBOARD: AtomicBool = AtomicBool::new(false);
pub static LAYERS: Mutex<Vec<(Vec<(i32, i32, Rgba<u8>)>, bool)>> = Mutex::new(Vec::new());
pub static SAVE_PATH: OnceCell<PathBuf> = OnceCell::new();
pub static COLOR: Mutex<[u8; 4]> = Mutex::new([128, 0, 128, 255]);
// pub static SAVE_PATH: OnceCell<PathBuf> = OnceCell::new();
//

static mut WINDOW: Option<ApplicationWindow> = None;
pub static mut SETTINGS_BOX: Option<Box> = None;
pub static mut ACTIVE_LAYER: Option<*mut Vec<(i32, i32, Rgba<u8>)>> = None;
static mut LAST_FRAME: (i32, i32) = (-1, -1);

fn add_layer() {
    let array = Vec::new();
    let mut layers = LAYERS.lock().unwrap();
    layers.push((array, true));
    let Some(vec) = layers.last_mut() else {
        panic!("how??")
    };
    let vec_ptr = &raw mut vec.0;

    unsafe {
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
    let wbox = Box::new(gtk4::Orientation::Horizontal, 33);
    main_box.append(&wbox);
    let color_picker_button = Button::builder()
        .icon_name("color-picker")
        .margin_top(10)
        .valign(gtk4::Align::Start)
        .build();
    color_picker_button.connect_clicked(color_picker);
    let adj = Adjustment::builder()
        .value(3.0)
        .lower(0.0)
        .upper(32.0)
        .step_increment(1.0)
        .build();
    let size_input = SpinButton::new(Some(&adj), 1.0, 0);
    size_input.set_value(3.0);
    size_input.set_valign(gtk4::Align::Start);
    size_input.set_margin_top(10);
    wbox.append(&color_picker_button);
    wbox.append(&size_input);
    wbox.set_visible(false);
    size_input.connect_value_changed(changed_size);

    unsafe {
        SETTINGS_BOX = Some(wbox);
    }

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
    IMG_WIDTH.store((width - 1) as i32, std::sync::atomic::Ordering::Relaxed);
    IMG_HEIGHT.store((height - 1) as i32, std::sync::atomic::Ordering::Relaxed);
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
    let mut lock = WIDGET_SIZE.lock().unwrap();
    lock.0 = img.width();
    lock.1 = img.height();
    drop(lock);
    unsafe {
        PICTURE_WIDGET = Some(img);
    }
    thread::spawn(|| {
        update_image();
    });

    window.add_controller(key_controller);

    window.present();
    unsafe {
        WINDOW = Some(window);
    }
}
