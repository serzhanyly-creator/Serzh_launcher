use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct Cfg {
    accent: u8,
    vol: f32,
    launch_cmd: String,
    mc_dir: String,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            accent: 0,
            vol: 0.5,
            launch_cmd: String::new(),
            mc_dir: String::new(),
        }
    }
}

struct App {
    tab: u8,
    cfg: Cfg,
    f_nick: String,
    f_path: String,
    focus: String,
    kb: bool,
    toast: String,
    toast_t: f32,
    anims: HashMap<String, f32>,
    sounds: HashMap<String, Sound>,
    t: f32,
    dt: f32,
    parts: Vec<(f32, f32, f32)>,
}

impl App {
    async fn new() -> Self {
        let mut a = Self {
            tab: 0,
            cfg: Cfg::default(),
            f_nick: String::new(),
            f_path: String::new(),
            focus: String::new(),
            kb: false,
            toast: String::new(),
            toast_t: 0.0,
            anims: HashMap::new(),
            sounds: HashMap::new(),
            t: 0.0,
            dt: 0.016,
            parts: vec![(0.0, 0.0, 0.0); 20],
        };
        for n in ["click", "toggle_on", "toggle_off", "open", "close", "launch", "error"] {
            for pre in ["assets/sounds/", "sounds/"] {
                if let Ok(s) = load_sound(&format!("{}{}.ogg", pre, n)).await {
                    a.sounds.insert(n.to_string(), s);
                    break;
                }
            }
        }
        a
    }
    fn snd(&self, n: &str) {
        if self.cfg.vol > 0.01 {
            if let Some(s) = self.sounds.get(n) {
                play_sound_once(s);
            }
        }
    }
    fn anim(&mut self, k: &str, target: f32) -> f32 {
        let cur = self.anims.entry(k.to_string()).or_insert(0.0);
        *cur += (target - *cur) * 0.15;
        *cur
    }
}

fn c(r: f32, g: f32, b: f32) -> Color { Color::new(r, g, b, 1.0) }
fn ca(col: Color, a: f32) -> Color { Color::new(col.r, col.g, col.b, a) }
fn pal() -> (Color, Color, Color, Color, Color) {
    (c(0.06, 0.04, 0.09), c(0.11, 0.09, 0.16), c(0.16, 0.14, 0.22), c(0.93, 0.91, 0.99), c(0.62, 0.58, 0.72))
}
fn accents(a: u8) -> (Color, Color) {
    match a {
        0 => (c(0.56, 0.36, 0.98), c(0.82, 0.58, 1.0)),
        1 => (c(0.98, 0.42, 0.56), c(1.0, 0.72, 0.82)),
        2 => (c(0.32, 0.82, 0.62), c(0.62, 1.0, 0.82)),
        3 => (c(0.42, 0.72, 0.98), c(0.72, 0.88, 1.0)),
        _ => (c(0.98, 0.72, 0.32), c(1.0, 0.88, 0.62)),
    }
}
fn narrow() -> bool { screen_width() < screen_height() * 0.85 }
fn area() -> (f32, f32, f32, f32) {
    let (w, h) = (screen_width(), screen_height());
    if narrow() {
        let pw = w * 0.92;
        (w * 0.04, 110.0, pw, h - 170.0)
    } else {
        let pw = w * 0.58;
        (w * 0.38, 90.0, pw, h - 140.0)
    }
}
fn rr(x: f32, y: f32, w: f32, h: f32, r: f32, col: Color) {
    draw_rectangle(x, y, w, h, col);
    draw_circle(x + r, y + r, r, col);
    draw_circle(x + w - r, y + r, r, col);
    draw_circle(x + r, y + h - r, r, col);
    draw_circle(x + w - r, y + h - r, r, col);
}
fn glow(x: f32, y: f32, r: f32, col: Color) {
    for i in 0..4 {
        draw_circle(x, y, r * (1.0 - i as f32 * 0.22), ca(col, col.a * 0.10));
    }
}
fn txt(s: &str, x: f32, y: f32, size: f32, col: Color) {
    draw_text(s, x, y, size, col);
}
fn tw(s: &str, size: f32) -> f32 {
    measure_text(s, None, size as u16, 1.0).width
}
fn hit(x: f32, y: f32, w: f32, h: f32) -> bool {
    let (mx, my) = mouse_position();
    mx >= x && mx <= x + w && my >= y && my <= y + h
}
fn btn(app: &mut App, key: &str, label: &str, x: f32, y: f32, w: f32, h: f32, active: bool) -> bool {
    let hover = hit(x, y, w, h);
    let (a1, a2) = accents(app.cfg.accent);
    let target = if hover && active { 1.0 } else if active { 0.6 } else { 0.2 };
    let v = app.anim(key, target);
    let (bg, _, _, _, _) = pal();
    rr(x, y, w, h, 10.0, bg);
    rr(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 9.0, ca(a1, v * 0.22));
    draw_rectangle_lines(x, y, w, h, 1.5, ca(a2, v));
    txt(label, x + 14.0, y + h * 0.63, 15.0, if active { c(0.93, 0.91, 0.99) } else { c(0.42, 0.38, 0.52) });
    if hover && is_mouse_button_pressed(MouseButton::Left) && active {
        app.snd("click");
        return true;
    }
    false
}
fn header(app: &mut App) {
    let (_, _, _, tx, _) = pal();
    txt("Serch Client", 28.0, 48.0, 26.0, tx);
    txt("Minecraft Launcher", 28.0, 72.0, 12.0, c(0.62, 0.58, 0.72));
    let w = screen_width();
    rr(w - 120.0, 22.0, 92.0, 28.0, 14.0, c(0.11, 0.09, 0.16));
    draw_circle(w - 102.0, 36.0, 5.0, c(0.13, 0.77, 0.37));
    txt("Online", w - 92.0, 40.0, 11.0, c(0.13, 0.77, 0.37));
}
fn nav(app: &mut App) {
    let (a1, _) = accents(app.cfg.accent);
    let labels = ["Главная", "Модули", "Аккаунты", "Настройки"];
    let (x, y, _, _) = area();
    let ny = if narrow() { 20.0 } else { y - 46.0 };
    let step = if narrow() { screen_width() * 0.92 / 4.0 } else { 120.0 };
    for (i, lab) in labels.iter().enumerate() {
        let nx = x + i as f32 * step;
        let active = app.tab == i as u8;
        txt(lab, nx + 8.0, ny + 18.0, 13.0, if active { c(0.93, 0.91, 0.99) } else { c(0.62, 0.58, 0.72) });
        if active {
            let lw = tw(lab, 13.0);
            draw_rectangle(nx + 6.0, ny + 24.0, lw, 2.0, a1);
        }
        if hit(nx, ny, step, 28.0) && is_mouse_button_pressed(MouseButton::Left) && app.tab != i as u8 {
            app.tab = i as u8;
            app.snd("click");
        }
    }
}
fn home(app: &mut App) -> PathBuf {
    if cfg!(target_os = "android") { android_dir() }
    else { std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(".")) }
}
fn tab_home(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, bg, card, tx, sub) = pal();
    rr(x, y, w, h, 16.0, bg);
    txt("Добро пожаловать", x + 24.0, y + 38.0, 20.0, tx);
    txt("Настрой профиль и запусти игру", x + 24.0, y + 62.0, 12.0, sub);
}
fn modules(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, bg, _, tx, sub) = pal();
    rr(x, y, w, h, 16.0, bg);
    txt("Модули", x + 24.0, y + 38.0, 20.0, tx);
}
fn accounts(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, bg, _, tx, _) = pal();
    rr(x, y, w, h, 16.0, bg);
    txt("Аккаунты", x + 24.0, y + 38.0, 20.0, tx);
}
fn settings(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, bg, _, tx, _) = pal();
    rr(x, y, w, h, 16.0, bg);
    txt("Настройки", x + 24.0, y + 38.0, 20.0, tx);
}
fn info(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, bg, _, tx, _) = pal();
    rr(x, y, w, h, 16.0, bg);
    txt("О программе", x + 24.0, y + 38.0, 20.0, tx);
}
fn launch(app: &mut App) {
    if app.f_nick.is_empty() {
        app.toast = "Укажи ник перед запуском".into();
        app.toast_t = 2.5;
        app.snd("error");
    }
}
#[cfg(target_os = "android")]
fn android_dir() -> PathBuf { PathBuf::from("/sdcard/Android/data/net.kdt.pojavlaunch/files") }

#[cfg(target_os = "android")]
fn open_pojav() -> bool { false }

#[cfg(not(target_os = "android"))]
fn open_pojav() -> bool { false }

#[cfg(target_os = "android")]
fn open_url(url: &str) {}

#[cfg(not(target_os = "android"))]
fn open_url(url: &str) { let _ = std::process::Command::new("xdg-open").arg(url).spawn(); }

fn window_conf() -> Conf {
    Conf {
        window_title: "Serch Client".to_string(),
        window_width: 1280,
        window_height: 800,
        fullscreen: false,
        high_dpi: false,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;
    loop {
        app.dt = get_frame_time().clamp(0.001, 0.1);
        app.t += app.dt;
        let (bg, _, _, _, _) = pal();
        clear_background(bg);
        header(&mut app);
        nav(&mut app);
        match app.tab {
            0 => tab_home(&mut app),
            1 => modules(&mut app),
            2 => accounts(&mut app),
            3 => settings(&mut app),
            _ => info(&mut app),
        }
        next_frame().await;
    }
}
