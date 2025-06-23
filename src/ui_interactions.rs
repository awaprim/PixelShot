use gdk4::{Key, ModifierType, glib::Propagation};
use gtk4::{
    Button, ColorChooserDialog, EventControllerKey, EventControllerMotion,
    prelude::{ColorChooserExt, DialogExtManual, GtkWindowExt},
};

use gtk4::{glib, prelude::*};

use crate::{
    CHANGED, COLOR, COPY_TO_CLIPBOARD, IMG_HEIGHT, IMG_WIDTH, LAST_FRAME, LAYERS, NEEDS_FULL,
    PICTURE_WIDGET, QUEUE, WINDOW, add_layer, draw_line::draw_line,
};

pub fn menu_button(_: &Button) {
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
}

pub fn key_pressed(
    _: &EventControllerKey,
    key: Key,
    _: u32,
    modifier: ModifierType,
) -> Propagation {
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
}

pub fn mouse_move(controller: &EventControllerMotion, x: f64, y: f64) {
    if controller
        .current_event_state()
        .eq(&ModifierType::BUTTON1_MASK)
    {
        let (w, h) = unsafe {
            let pic = &raw const PICTURE_WIDGET;
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
}

pub fn copy_to_clipbard_button(_: &Button) {
    unsafe {
        CHANGED = true;
        COPY_TO_CLIPBOARD = true;
    }
}
