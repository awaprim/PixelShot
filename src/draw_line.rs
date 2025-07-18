use std::collections::VecDeque;

use crate::QUEUE;

pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32) {
    let mut lock = QUEUE.lock().unwrap();
    if (y1 - y0).abs() < (x1 - x0).abs() {
        if x0 > x1 {
            draw_line_low(x1, y1, x0, y0, &mut lock);
        } else {
            draw_line_low(x0, y0, x1, y1, &mut lock);
        }
    } else if y0 > y1 {
        draw_line_high(x1, y1, x0, y0, &mut lock);
    } else {
        draw_line_high(x0, y0, x1, y1, &mut lock);
    }
    lock.push_back((x0, y0));
    lock.push_back((x1, y1));
    // (&mut *&raw mut QUEUE).push_back((x0, y0));
    // (&mut *&raw mut QUEUE).push_back((x1, y1));
}

fn draw_line_high(x0: i32, y0: i32, x1: i32, y1: i32, queue: &mut VecDeque<(i32, i32)>) {
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
        queue.push_back((x, y));
        if d > 0 {
            x += xi;
            d += 2 * (dx - dy);
        } else {
            d += 2 * dx;
        }
    }
}
fn draw_line_low(x0: i32, y0: i32, x1: i32, y1: i32, queue: &mut VecDeque<(i32, i32)>) {
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
        queue.push_back((x, y));
        if d > 0 {
            y += yi;
            d += 2 * (dy - dx);
        } else {
            d += 2 * dy;
        }
    }
}
