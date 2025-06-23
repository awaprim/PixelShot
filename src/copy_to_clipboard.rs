use std::{
    io::{Cursor, Write},
    process::{Command, Stdio},
};

use image::DynamicImage;

pub fn copy_to_clipbard(img: &DynamicImage) {
    let mut encoded_bytes = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut encoded_bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    let os = std::env::consts::OS;
    match os {
        "linux" => {
            let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") else {
                panic!("unknown session type");
            };
            match session_type.as_str() {
                "x11" => {
                    println!("unimplemented");
                }
                "wayland" => {
                    copy_to_clipboard_linux_wayland(encoded_bytes);
                }
                _ => {
                    println!("unimplemented");
                }
            }
        }
        _ => {
            println!("unimplemented");
        }
    }
}

fn copy_to_clipboard_linux_wayland(bytes: Vec<u8>) {
    let cmd = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = cmd.stdin.unwrap();
    stdin.write_all(&bytes).unwrap();
}
