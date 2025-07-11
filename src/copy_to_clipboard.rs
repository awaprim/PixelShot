use std::{
    fs::File,
    io::{Cursor, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use image::DynamicImage;

pub fn copy_to_clipbard(img: &DynamicImage, file_path: Option<PathBuf>) {
    let mut encoded_bytes = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut encoded_bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    if let Some(path) = file_path {
        let success = save_to_file(path, &encoded_bytes);
        if success.is_err() {
            println!("failed saving to file");
        }
    }
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
fn save_to_file(mut path: PathBuf, img: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let current_date = chrono::Local::now();
    path.push(current_date.to_string() + ".png");
    let mut file = File::create_new(path)?;
    file.write_all(img)?;
    file.sync_all()?;
    Ok(())
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
