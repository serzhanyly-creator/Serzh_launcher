use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const CARD_W: f32 = 220.0;
const CARD_H: f32 = 120.0;
const CATS: [&str; 5] = ["Все", "Combat", "Movement", "Render", "Misc"];
const TABS: [&str; 5] = ["Главная", "Модули", "Аккаунты", "Настройки", "Инфо"];
const MODULES: [(&str, &str, &str); 12] = [
    ("KillAura", "Combat", "Автоматическая атака по врагам"),
    ("AutoDodge", "Combat", "Уклонение от снарядов и атак"),
    ("Reach", "Combat", "Увеличенная дистанция атаки"),
    ("AntiBot", "Combat", "Игнорировать ботов"),
    ("Fly", "Movement", "Полёт в любом месте"),
    ("Speed", "Movement", "Ускорение движения"),
    ("NoFall", "Movement", "Защита от урона при падении"),
    ("Step", "Movement", "Подъём на блоки"),
    ("ESP", "Render", "Подсветка игроков и предметов"),
    ("XRay", "Render", "Прозрачные блоки и руды"),
    ("Fullbright", "Render", "Яркость во тьме"),
    ("Timer", "Misc", "Ускорение игрового времени"),
];

#[derive(Clone, Serialize, Deserialize)]
struct Cfg { vol: f32, anim: bool, accent: i64, shared: String }
impl Default for Cfg {
    fn default() -> Self { Self { vol: 0.7, anim: true, accent: 0, shared: String::new() } }
}
#[derive(Clone, Serialize, Deserialize)]
struct Acc { name: String, uuid: String }
#[derive(Clone, Serialize, Deserialize, Default)]
struct Accs { accounts: Vec<Acc>, selected: String }

fn c(r: f32, g: f32, b: f32) -> Color { Color::new(r, g, b, 1.0) }
fn ca(col: Color, a: f32) -> Color { Color::new(col.r, col.g, col.b, a.clamp(0.0, 1.0)) }
fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0)
}
fn accents(a: i64) -> (Color, Color) {
    match a.clamp(0, 3) {
        0 => (c(0.49, 0.23, 0.93), c(0.66, 0.33, 0.97)),
        1 => (c(0.02, 0.71, 0.83), c(0.23, 0.51, 0.96)),
        2 => (c(0.98, 0.45, 0.09), c(0.93, 0.28, 0.60)),
        _ => (c(0.13, 0.77, 0.37), c(0.06, 0.73, 0.51)),
    }
}
fn narrow() -> bool { screen_width() < 700.0 }
fn rr(x: f32, y: f32, w: f32, h: f32, r: f32, col: Color) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    draw_rectangle(x + r, y, w - 2.0 * r, h, col);
    draw_rectangle(x, y + r, w, h - 2.0 * r, col);
    draw_circle(x + r, y + r, r, col);
    draw_circle(x + w - r, y + r, r, col);
    draw_circle(x + r, y + h - r, r, col);
    draw_circle(x + w - r, y + h - r, r, col);
}
fn glow(x: f32, y: f32, r: f32, col: Color) {
    for i in 0..4 { draw_circle(x, y, r * (1.0 - i as f32 * 0.22), ca(col, col.a * 0.10)); }
}
fn txt(s: &str, x: f32, y: f32, size: f32, col: Color) { draw_text(s, x, y, size, col); }
fn tw(s: &str, size: f32) -> f32 { measure_text(s, None, size as u16).width }
fn hit(x: f32, y: f32, w: f32, h: f32) -> bool {
    let (mx, my) = mouse_position();
    mx >= x && mx <= x + w && my >= y && my <= y + h
}
fn tap(x: f32, y: f32, w: f32, h: f32) -> bool { hit(x, y, w, h) && is_mouse_button_pressed(MouseButton::Left) }
