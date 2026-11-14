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
fn pal() -> (Color, Color, Color, Color, Color) {
    (c(0.039, 0.024, 0.07), c(0.09, 0.06, 0.14), c(0.165, 0.106, 0.24), c(0.93, 0.914, 0.996), c(0.62, 0.56, 0.75))
}

struct App {
    cfg: Cfg, accs: Accs, enabled: Vec<String>,
    tab: usize, cat: usize, page: usize,
    f_nick: String, f_path: String, focus: String, kb: bool,
    toast: String, toast_t: f32,
    anims: HashMap<String, f32>, sounds: HashMap<String, Sound>,
    t: f32, dt: f32, parts: Vec<(f32, f32, f32)>,
}

fn home() -> PathBuf {
    if cfg!(target_os = "android") { android_dir() }
    else { std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(".")) }
}
fn load<T: serde::de::DeserializeOwned + Default>(p: PathBuf) -> T {
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save<T: Serialize>(p: PathBuf, v: &T) {
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::write(p, serde_json::to_string_pretty(v).unwrap_or_default());
}
fn uuid_of(name: &str) -> String {
    let mut b = md5::compute(format!("OfflinePlayer:{}", name).as_bytes()).0;
    b[6] = (b[6] & 0x0f) | 0x30;
    b[8] = (b[8] & 0x3f) | 0x80;
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

impl App {
    fn shared(&self) -> PathBuf {
        if !self.cfg.shared.trim().is_empty() { return PathBuf::from(self.cfg.shared.trim()); }
        if cfg!(target_os = "android") {
            let p = PathBuf::from("/storage/emulated/0/PojavLauncher/.minecraft/config/searchclient");
            if p.exists() { return p; }
            return home().join("searchclient");
        }
        home().join(".minecraft/config/searchclient")
    }
    fn new() -> App {
        let cfg: Cfg = load(home().join("serch-config.json"));
        let mut a = App {
            cfg, accs: Accs::default(), enabled: vec![],
            tab: 0, cat: 0, page: 0,
            f_nick: String::new(), f_path: String::new(), focus: String::new(), kb: false,
            toast: String::new(), toast_t: 0.0,
            anims: HashMap::new(), sounds: HashMap::new(),
            t: 0.0, dt: 0.016,
            parts: (0..40).map(|i| (((i * 53) % 100) as f32 / 100.0, ((i * 37) % 100) as f32 / 100.0, 0.2 + ((i * 29) % 80) as f32 / 100.0)).collect(),
        };
        a.f_path = a.cfg.shared.clone();
        a.accs = load(a.shared().join("accounts.json"));
        a.enabled = load(a.shared().join("modules.json"));
        for n in ["click", "toggle_on", "toggle_off", "open", "close", "launch", "error"] {
            for pre in ["assets/sounds/", "sounds/"] {
                if let Ok(s) = load_sound(&format!("{}{}.ogg", pre, n)) { a.sounds.insert(n.to_string(), s); break; }
            }
        }
        a
    }
    fn snd(&self, n: &str) { if self.cfg.vol > 0.01 { if let Some(s) = self.sounds.get(n) { play_sound_once(s); } } }
    fn anim(&mut self, k: &str, target: f32) -> f32 {
        let sp = if self.cfg.anim { 12.0 } else { 1000.0 };
        let v = self.anims.entry(k.to_string()).or_insert(target);
        *v += (target - *v) * (1.0 - (-sp * self.dt).exp());
        *v
    }
    fn say(&mut self, m: &str) { self.toast = m.to_string(); self.toast_t = 2.5; }
    fn save_all(&self) {
        save(home().join("serch-config.json"), &self.cfg);
        save(self.shared().join("accounts.json"), &self.accs);
        save(self.shared().join("modules.json"), &self.enabled);
    }
    fn acc_name(&self) -> String { if self.accs.selected.is_empty() { "Steve".to_string() } else { self.accs.selected.clone() } }
}

fn btn(app: &mut App, id: &str, label: &str, x: f32, y: f32, w: f32, h: f32, primary: bool) -> bool {
    let hov = hit(x, y, w, h);
    let a = app.anim(id, if hov { 1.0 } else { 0.0 });
    let (a1, a2) = accents(app.cfg.accent);
    let (_, card, bd, tx, _) = pal();
    let press = hov && is_mouse_button_down(MouseButton::Left);
    let oy = if press { 1.5 } else { 0.0 };
    rr(x, y + oy + 2.0, w, h, 10.0, ca(c(0.0, 0.0, 0.0), 0.35));
    if primary { rr(x, y + oy, w, h, 10.0, mix(a1, a2, 0.45 + 0.3 * a)); }
    else {
        rr(x, y + oy, w, h, 10.0, mix(bd, a2, a * 0.6));
        rr(x + 1.0, y + oy + 1.0, w - 2.0, h - 2.0, 9.0, mix(card, c(0.13, 0.085, 0.2), a));
    }
    let wtx = tw(label, 13.0);
    txt(label, x + (w - wtx) / 2.0, y + oy + h / 2.0 + 5.0, 13.0, tx);
    if tap(x, y, w, h) { app.snd("click"); return true; }
    false
}
fn toggle(app: &mut App, id: &str, on: bool, x: f32, y: f32) -> Option<bool> {
    let t = app.anim(id, if on { 1.0 } else { 0.0 });
    let (a1, _) = accents(app.cfg.accent);
    let (_, _, _, tx, _) = pal();
    rr(x, y, 42.0, 22.0, 11.0, mix(c(0.22, 0.17, 0.3), a1, t));
    draw_circle(x + 11.0 + 20.0 * t, y + 11.0, 9.0, tx);
    if tap(x, y, 42.0, 22.0) { app.snd(if on { "toggle_off" } else { "toggle_on" }); return Some(!on); }
    None
}
fn slider(app: &mut App, id: &str, v: f32, mn: f32, mx: f32, x: f32, y: f32, w: f32) -> Option<f32> {
    let hov = hit(x, y - 8.0, w, 22.0);
    let drag = hov && is_mouse_button_down(MouseButton::Left);
    let (a1, a2) = accents(app.cfg.accent);
    let (_, _, _, tx, _) = pal();
    let mut out = None;
    let mut val = v;
    if drag {
        val = mn + (mx - mn) * ((mouse_position().0 - x) / w).clamp(0.0, 1.0);
        if (val - v).abs() > 0.0001 { out = Some(val); }
    }
    let t = ((val - mn) / (mx - mn).max(0.0001)).clamp(0.0, 1.0);
    rr(x, y, w, 6.0, 3.0, c(0.19, 0.14, 0.27));
    rr(x, y, (w * t).max(8.0), 6.0, 3.0, mix(a1, a2, t));
    let g = app.anim(id, if hov { 1.0 } else { 0.0 });
    draw_circle(x + w * t, y + 3.0, 7.0 + 2.0 * g, tx);
    out
}
fn field(app: &mut App, id: &str, x: f32, y: f32, w: f32, h: f32, ph: &str) {
    let focused = app.focus == id;
    if focused { app.kb = true; }
    let (_, a2) = accents(app.cfg.accent);
    let (_, card, bd, tx, dim) = pal();
    let mut s = if id == "nick" { app.f_nick.clone() } else { app.f_path.clone() };
    rr(x, y, w, h, 10.0, if focused { a2 } else { bd });
    rr(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 9.0, card);
    if tap(x, y, w, h) { app.focus = if focused { String::new() } else { id.to_string() }; app.snd("click"); }
    let empty = s.is_empty();
    let show = if empty { ph.to_string() } else { s.clone() };
    txt(&show, x + 12.0, y + h / 2.0 + 5.0, 13.0, if empty && !focused { dim } else { tx });
    if focused {
        while let Some(ch) = get_char_pressed() { if ch.is_ascii_graphic() || ch == ' ' { s.push(ch); } }
        if is_key_pressed(KeyCode::Backspace) { s.pop(); }
        if (app.t * 2.0) as i64 % 2 == 0 { draw_rectangle(x + 12.0 + tw(&show, 13.0) + 1.0, y + 8.0, 1.5, h - 16.0, a2); }
    } else { while get_char_pressed().is_some() {} }
    if id == "nick" { app.f_nick = s; } else { app.f_path = s; }
}
