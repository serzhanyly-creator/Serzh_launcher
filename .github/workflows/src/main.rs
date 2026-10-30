use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

const SIDEBAR_W: f32 = 210.0;
const CARD_W: f32 = 230.0;
const CARD_H: f32 = 164.0;

fn narrow() -> bool { screen_width() < 700.0 }
fn header_h() -> f32 { if narrow() { 50.0 } else { 56.0 } }
fn nav_h() -> f32 { if narrow() { 62.0 } else { 0.0 } }
fn status_h() -> f32 { if narrow() { 34.0 } else { 40.0 } }
fn content_x() -> f32 { if narrow() { 12.0 } else { SIDEBAR_W + 16.0 } }
fn content_y() -> f32 { header_h() + 12.0 }
fn content_w() -> f32 { screen_width() - content_x() * 2.0 }
fn content_h() -> f32 { screen_height() - content_y() - status_h() - nav_h() - 12.0 }

#[derive(Clone, Copy)]
struct Pal {
    bg: Color, sidebar: Color, card: Color, card_hov: Color, border: Color,
    text: Color, dim: Color, acc1: Color, acc2: Color, chip: Color,
    green: Color, red: Color, shadow: Color,
}

fn presets() -> [(Color, Color); 4] {
    [
        (Color::new(0.49, 0.23, 0.93, 1.0), Color::new(0.66, 0.33, 0.97, 1.0)),
        (Color::new(0.02, 0.71, 0.83, 1.0), Color::new(0.23, 0.51, 0.96, 1.0)),
        (Color::new(0.98, 0.45, 0.09, 1.0), Color::new(0.93, 0.28, 0.60, 1.0)),
        (Color::new(0.13, 0.77, 0.37, 1.0), Color::new(0.06, 0.73, 0.51, 1.0)),
    ]
}
const PRESET_NAMES: [&str; 4] = ["Aurora", "Neon", "Sunset", "Void"];

fn pal(accent: i64) -> Pal {
    let (a1, a2) = presets()[accent.clamp(0, 3) as usize];
    Pal {
        bg: Color::new(0.039, 0.024, 0.07, 1.0),
        sidebar: Color::new(0.043, 0.027, 0.08, 1.0),
        card: Color::new(0.09, 0.06, 0.14, 1.0),
        card_hov: Color::new(0.13, 0.085, 0.2, 1.0),
        border: Color::new(0.165, 0.106, 0.24, 1.0),
        text: Color::new(0.93, 0.914, 0.996, 1.0),
        dim: Color::new(0.62, 0.56, 0.75, 1.0),
        acc1: a1, acc2: a2,
        chip: Color::new(0.14, 0.086, 0.204, 1.0),
        green: Color::new(0.13, 0.77, 0.37, 1.0),
        red: Color::new(0.94, 0.27, 0.27, 1.0),
        shadow: Color::new(0.0, 0.0, 0.0, 0.35),
    }
}

fn mixc(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}
fn with_a(c: Color, a: f32) -> Color { Color::new(c.r, c.g, c.b, a.clamp(0.0, 1.0)) }
