#![allow(static_mut_refs)]
use gdk4::{Key, ModifierType, glib::Propagation};
use gtk4::{
    Button, ColorChooserDialog, EventControllerKey, EventControllerMotion, SpinButton,
    prelude::{ColorChooserExt, DialogExtManual, GtkWindowExt},
};

use gtk4::{glib, prelude::*};

use crate::{
    COLOR, COPY_TO_CLIPBOARD, IMG_HEIGHT, IMG_WIDTH, LAST_FRAME, LAYERS, NEEDS_FULL, QUEUE,
    SETTINGS_BOX, SIZE, WIDGET_SIZE, WINDOW, add_layer, draw_line::draw_line,
};

pub fn menu_button(_: &Button) {
    unsafe {
        let Some(settings) = &mut SETTINGS_BOX else {
            return;
        };
        settings.set_visible(!settings.get_visible());
    }
}
pub fn color_picker(_: &Button) {
    let window = unsafe {
        let Some(window) = &WINDOW else {
            panic!("how");
        };
        window
    };

    let picker = ColorChooserDialog::new(Some("pick color"), Some(window));
    picker.run_async(|picker, _resp| {
        let color = picker.rgba();
        let mut lock = COLOR.lock().unwrap();
        *lock = [
            (color.red() * 255.0) as u8,
            (color.green() * 255.0) as u8,
            (color.blue() * 255.0) as u8,
            (color.alpha() * 255.0) as u8,
        ];
        drop(lock);
        picker.close();
    });
}

pub fn key_pressed(
    _: &EventControllerKey,
    key: Key,
    _: u32,
    modifier: ModifierType,
) -> Propagation {
    if (key.eq(&Key::c) || key.eq(&Key::C)) && modifier.eq(&ModifierType::CONTROL_MASK) {
        COPY_TO_CLIPBOARD.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if (key.eq(&Key::z) || key.eq(&Key::Z))
        && modifier.eq(&(ModifierType::SHIFT_MASK | ModifierType::CONTROL_MASK))
    {
        let mut overlays = LAYERS.lock().unwrap();
        for (_, active) in overlays.iter_mut() {
            if !*active {
                *active = true;
                break;
            }
        }

        NEEDS_FULL.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if (key.eq(&Key::z) || key.eq(&Key::Z)) && modifier.eq(&ModifierType::CONTROL_MASK) {
        let mut overlays = LAYERS.lock().unwrap();
        for (_, active) in overlays.iter_mut().rev() {
            if *active {
                *active = false;
                break;
            }
        }

        NEEDS_FULL.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    glib::Propagation::Proceed
}

pub fn mouse_move(controller: &EventControllerMotion, x: f64, y: f64) {
    if controller
        .current_event_state()
        .eq(&ModifierType::BUTTON1_MASK)
    {
        let (w, h) = {
            let lock = WIDGET_SIZE.lock().unwrap();
            *lock
        };
        let (real_height, real_width) = {
            let img_height = IMG_HEIGHT.load(std::sync::atomic::Ordering::Relaxed);
            let img_width = IMG_WIDTH.load(std::sync::atomic::Ordering::Relaxed);
            (
                (((y / h as f64).min(1.0) * (img_height) as f64) as i32).max(0),
                (((x / w as f64).min(1.0) * (img_width) as f64) as i32).max(0),
            )
        };
        unsafe {
            if LAST_FRAME.0 != -1 {
                draw_line(LAST_FRAME.0, LAST_FRAME.1, real_width, real_height);
            } else {
                add_layer();
                let mut lock = QUEUE.lock().unwrap();
                lock.push_back((real_width, real_height));
            }
            LAST_FRAME = (real_width, real_height);
        }
    } else {
        unsafe {
            LAST_FRAME = (-1, -1);
        }
    };
}
pub fn changed_size(button: &SpinButton) {
    let new_size = button.value();
    SIZE.store(new_size as u32, std::sync::atomic::Ordering::Relaxed);
}

pub fn copy_to_clipbard_button(_: &Button) {
    COPY_TO_CLIPBOARD.store(true, std::sync::atomic::Ordering::Relaxed);
}
