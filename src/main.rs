use macroquad::prelude::*;
use macroquad::audio::{Sound, load_sound, play_sound_once, PlaySoundParams};
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
fn tw(s: &str, size: f32) -> f32 { measure_text(s, None, size as u16, 1.0).width }
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
    async fn new() -> App {
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
                if let Ok(s) = load_sound(&format!("{}{}.ogg", pre, n)).await { a.sounds.insert(n.to_string(), s); break; }
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
fn area() -> (f32, f32, f32, f32) {
    let h = screen_height();
    let hh = if narrow() { 50.0 } else { 56.0 };
    let nh = if narrow() { 62.0 } else { 0.0 };
    let x = if narrow() { 12.0 } else { 226.0 };
    let y = hh + 12.0;
    let w = screen_width() - x - 12.0;
    (x, y, w, h - y - nh - 40.0 - 12.0)
}
fn header(app: &mut App) {
    let (bg, card, _, tx, _) = pal();
    let (a1, a2) = accents(app.cfg.accent);
    let w = screen_width();
    let h = if narrow() { 50.0 } else { 56.0 };
    draw_rectangle(0.0, 0.0, w, h, bg);
    draw_rectangle(0.0, h - 1.0, w, 1.0, ca(a1, 0.3));
    glow(24.0, h / 2.0, 40.0, ca(a1, 0.6));
    rr(12.0, h / 2.0 - 12.0, 24.0, 24.0, 7.0, a1);
    draw_triangle(vec2(18.0, h / 2.0 - 6.0), vec2(18.0, h / 2.0 + 6.0), vec2(30.0, h / 2.0), tx);
    txt("SERCH CLIENT", 44.0, h / 2.0 + 6.0, if narrow() { 14.0 } else { 17.0 }, tx);
    let name = app.acc_name();
    let nw = tw(&name, 12.0);
    rr(w - nw - 44.0, h / 2.0 - 15.0, nw + 30.0, 30.0, 15.0, card);
    draw_circle(w - nw - 30.0, h / 2.0, 8.0, a2);
    txt(&name, w - nw - 16.0, h / 2.0 + 4.0, 12.0, tx);
}
fn nav(app: &mut App) {
    let (bg, _, _, tx, dim) = pal();
    let (a1, _) = accents(app.cfg.accent);
    let w = screen_width(); let h = screen_height();
    if narrow() {
        let y = h - 62.0;
        draw_rectangle(0.0, y, w, 62.0, bg);
        draw_rectangle(0.0, y, w, 1.0, ca(a1, 0.3));
        let iw = w / 5.0;
        let p = app.anim("nav", app.tab as f32);
        rr(iw * p + iw / 2.0 - 14.0, y + 6.0, 28.0, 28.0, 9.0, ca(a1, 0.85));
        for i in 0..5 {
            let x = iw * i as f32;
            let sel = i == app.tab;
            let lw = tw(TABS[i], 8.0);
            txt(TABS[i], x + iw / 2.0 - lw / 2.0, y + 52.0, 8.0, if sel { tx } else { dim });
            draw_circle(x + iw / 2.0, y + 20.0, if sel { 5.0 } else { 3.0 }, if sel { tx } else { dim });
            if tap(x, y, iw, 62.0) { app.tab = i; app.page = 0; app.snd("click"); }
        }
    } else {
        draw_rectangle(0.0, 56.0, 210.0, h - 96.0, bg);
        let p = app.anim("nav", app.tab as f32);
        rr(10.0, 76.0 + p * 46.0, 190.0, 40.0, 12.0, ca(a1, 0.85));
        for i in 0..5 {
            let y = 76.0 + i as f32 * 46.0;
            let sel = i == app.tab;
            txt(TABS[i], 52.0, y + 26.0, 14.0, if sel { tx } else { dim });
            draw_circle(30.0, y + 20.0, if sel { 5.0 } else { 3.0 }, if sel { tx } else { dim });
            if tap(10.0, y, 190.0, 40.0) { app.tab = i; app.page = 0; app.snd("click"); }
        }
    }
}
fn tab_home(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, card, bd, tx, dim) = pal();
    let (a1, a2) = accents(app.cfg.accent);
    let ch = h.min(260.0);
    rr(x, y + 3.0, w, ch, 18.0, ca(c(0.0, 0.0, 0.0), 0.35));
    rr(x, y, w, ch, 18.0, bd);
    rr(x + 1.0, y + 1.0, w - 2.0, ch - 2.0, 17.0, card);
    glow(x + w * 0.8, y + 40.0, 150.0, ca(a1, 0.8));
    let an = app.cfg.anim;
    for p in app.parts.iter_mut() { if an { p.1 -= 0.0015 * p.2; if p.1 < -0.05 { p.1 = 1.05; } } }
    for p in app.parts.iter() { draw_circle(x + p.0 * w, y + p.1 * ch, 1.0 + 2.0 * p.2, ca(a2, 0.18 * p.2)); }
    txt("Добро пожаловать в", x + 24.0, y + 50.0, 12.0, dim);
    txt("SERCH CLIENT", x + 24.0, y + 100.0, if narrow() { 30.0 } else { 42.0 }, tx);
    txt("Покоряй мир, будь на шаг впереди.", x + 26.0, y + 128.0, 14.0, a2);
    if btn(app, "hp", "Запустить игру", x + 24.0, y + ch - 62.0, 180.0, 44.0, true) { launch(app); }
    if btn(app, "hs", "Настройки", x + 216.0, y + ch - 62.0, 140.0, 44.0, false) { app.tab = 3; app.snd("open"); }
    let sy = y + ch + 16.0;
    let rows = [("Minecraft", "1.21.4".to_string()), ("Модулей включено", app.enabled.len().to_string()), ("Аккаунт", app.acc_name())];
    let rh = rows.len() as f32 * 30.0 + 40.0;
    rr(x, sy, w, rh, 14.0, bd);
    rr(x + 1.0, sy + 1.0, w - 2.0, rh - 2.0, 13.0, card);
    txt("Статус", x + 14.0, sy + 26.0, 14.0, tx);
    for (i, r) in rows.iter().enumerate() {
        txt(r.0, x + 14.0, sy + 50.0 + i as f32 * 28.0, 12.0, dim);
        let vw = tw(&r.1, 12.0);
        txt(&r.1, x + w - vw - 14.0, sy + 50.0 + i as f32 * 28.0, 12.0, tx);
    }
}
fn modules(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, card, bd, tx, dim) = pal();
    let (a1, a2) = accents(app.cfg.accent);
    txt("Модули", x, y + 20.0, 22.0, tx);
    let mut cx = x; let mut cy = y + 32.0;
    for i in 0..CATS.len() {
        let label = CATS[i];
        let cw = tw(label, 12.0) + 28.0;
        if cx + cw > x + w { cx = x; cy += 38.0; }
        let sel = i == app.cat;
        let hov = hit(cx, cy, cw, 32.0);
        let a = app.anim(&format!("cat{}", i), if hov || sel { 1.0 } else { 0.0 });
        if sel { rr(cx, cy, cw, 32.0, 10.0, mix(a1, a2, 0.5)); }
        else { rr(cx, cy, cw, 32.0, 10.0, mix(bd, a2, a * 0.5)); rr(cx + 1.0, cy + 1.0, cw - 2.0, 30.0, 9.0, card); }
        txt(label, cx + 14.0, cy + 21.0, 12.0, if sel { tx } else { dim });
        if tap(cx, cy, cw, 32.0) { app.cat = i; app.page = 0; app.snd("click"); }
        cx += cw + 8.0;
    }
    let gy = cy + 42.0;
    let gh = (y + h - gy - 46.0).max(80.0);
    let list: Vec<usize> = MODULES.iter().enumerate().filter(|(_, m)| app.cat == 0 || m.1 == CATS[app.cat]).map(|(i, _)| i).collect();
    let cols = ((w + 12.0) / (CARD_W + 12.0)).max(1.0) as usize;
    let rows = ((gh + 12.0) / (CARD_H + 12.0)).max(1.0) as usize;
    let per = (cols * rows).max(1);
    let pages = ((list.len() + per - 1) / per).max(1);
    if app.page >= pages { app.page = pages - 1; }
    let start = app.page * per;
    let end = (start + per).min(list.len());
    for n in start..end {
        let idx = list[n];
        let m = MODULES[idx];
        let k = n - start;
        let cx2 = x + (k % cols) as f32 * (CARD_W + 12.0);
        let cy2 = gy + (k / cols) as f32 * (CARD_H + 12.0);
        let hov = hit(cx2, cy2, CARD_W, CARD_H);
        let a = app.anim(&format!("m{}", idx), if hov { 1.0 } else { 0.0 });
        rr(cx2, cy2 + 3.0, CARD_W, CARD_H, 14.0, ca(c(0.0, 0.0, 0.0), 0.35));
        rr(cx2, cy2, CARD_W, CARD_H, 14.0, mix(bd, a2, a * 0.6));
        rr(cx2 + 1.0, cy2 + 1.0, CARD_W - 2.0, CARD_H - 2.0, 13.0, mix(card, c(0.13, 0.085, 0.2), a));
        txt(m.0, cx2 + 14.0, cy2 + 30.0, 15.0, tx);
        txt(m.2, cx2 + 14.0, cy2 + 52.0, 11.0, dim);
        let chw = tw(m.1, 10.0) + 16.0;
        rr(cx2 + CARD_W - chw - 12.0, cy2 + 14.0, chw, 20.0, 8.0, c(0.14, 0.086, 0.204));
        txt(m.1, cx2 + CARD_W - chw - 4.0, cy2 + 28.0, 10.0, dim);
        let on = app.enabled.iter().any(|e| e == m.0);
        if let Some(new) = toggle(app, &format!("t{}", idx), on, cx2 + 14.0, cy2 + CARD_H - 34.0) {
            if new { app.enabled.push(m.0.to_string()); } else { app.enabled.retain(|e| e != m.0); }
            app.save_all();
        }
    }
    let by = y + h - 40.0;
    if btn(app, "prev", "Назад", x, by, 100.0, 34.0, false) { if app.page > 0 { app.page -= 1; } }
    let pl = format!("{}/{}", app.page + 1, pages);
    let pw = tw(&pl, 12.0);
    txt(&pl, x + w / 2.0 - pw / 2.0, by + 22.0, 12.0, dim);
    if btn(app, "next", "Вперёд", x + w - 100.0, by, 100.0, 34.0, false) { if app.page + 1 < pages { app.page += 1; } }
}
fn accounts(app: &mut App) {
    let (x, y, w, h) = area();
    let (_, card, bd, tx, dim) = pal();
    let (_, a2) = accents(app.cfg.accent);
    txt("Аккаунты", x, y + 20.0, 22.0, tx);
    field(app, "nick", x, y + 32.0, w - 130.0, 36.0, "Никнейм...");
    if btn(app, "add", "Добавить", x + w - 120.0, y + 32.0, 120.0, 36.0, true) {
        let name = app.f_nick.trim().to_string();
        if name.is_empty() || app.accs.accounts.iter().any(|a| a.name.eq_ignore_ascii_case(&name)) { app.snd("error"); app.say("Пустой или уже есть"); }
        else {
            app.accs.accounts.push(Acc { name: name.clone(), uuid: uuid_of(&name) });
            if app.accs.selected.is_empty() { app.accs.selected = name.clone(); }
            app.f_nick.clear();
            app.save_all();
            app.say("Аккаунт добавлен");
        }
    }
    let ly = y + 80.0;
    let lh = (y + h - ly - 46.0).max(60.0);
    let per = ((lh + 8.0) / 56.0).max(1.0) as usize;
    let total = app.accs.accounts.len();
    let pages = ((total + per - 1) / per).max(1);
    if app.page >= pages { app.page = pages - 1; }
    let start = app.page * per;
    let end = (start + per).min(total);
    for i in start..end {
        let a = app.accs.accounts[i].clone();
        let ry = ly + (i - start) as f32 * 56.0;
        let sel = a.name == app.accs.selected;
        let hov = hit(x, ry, w, 48.0);
        let an = app.anim(&format!("a{}", i), if hov { 1.0 } else { 0.0 });
        rr(x, ry + 3.0, w, 48.0, 12.0, ca(c(0.0, 0.0, 0.0), 0.35));
        rr(x, ry, w, 48.0, 12.0, if sel { a2 } else { mix(bd, a2, an * 0.5) });
        rr(x + 1.0, ry + 1.0, w - 2.0, 46.0, 11.0, mix(card, c(0.13, 0.085, 0.2), an));
        draw_circle(x + 26.0, ry + 24.0, 13.0, if sel { a2 } else { c(0.14, 0.086, 0.204) });
        txt(&a.name, x + 48.0, ry + 22.0, 14.0, if sel { a2 } else { tx });
        let us = &a.uuid[..16.min(a.uuid.len())];
        txt(us, x + 48.0, ry + 38.0, 10.0, dim);
        if btn(app, &format!("sel{}", i), "Выбрать", x + w - 160.0, ry + 8.0, 100.0, 32.0, sel) { app.accs.selected = a.name.clone(); app.save_all(); app.say("Выбран аккаунт"); }
        if btn(app, &format!("del{}", i), "X", x + w - 52.0, ry + 8.0, 40.0, 32.0, false) {
            app.accs.accounts.retain(|z| z.name != a.name);
            if app.accs.selected == a.name { app.accs.selected.clear(); }
            app.save_all(); app.snd("close");
            break;
        }
    }
    let by = y + h - 40.0;
    if btn(app, "aprev", "Назад", x, by, 100.0, 34.0, false) { if app.page > 0 { app.page -= 1; } }
    if btn(app, "anext", "Вперёд", x + w - 100.0, by, 100.0, 34.0, false) { if app.page + 1 < pages { app.page += 1; } }
}
fn settings(app: &mut App) {
    let (x, y, w, _h) = area();
    let (_, card, bd, tx, dim) = pal();
    txt("Настройки", x, y + 20.0, 22.0, tx);
    let pw = w.min(640.0);
    let ph = 430.0;
    rr(x, y + 32.0, pw, ph, 14.0, bd);
    rr(x + 1.0, y + 33.0, pw - 2.0, ph - 2.0, 13.0, card);
    let mut yy = y + 48.0;
    txt("Громкость", x + 16.0, yy + 20.0, 12.0, dim);
    if let Some(v) = slider(app, "vol", app.cfg.vol, 0.0, 1.0, x + 150.0, yy + 14.0, pw - 240.0) { app.cfg.vol = v; }
    yy += 40.0;
    txt("Анимации", x + 16.0, yy + 20.0, 12.0, dim);
    if let Some(v) = toggle(app, "anim", app.cfg.anim, x + 150.0, yy + 6.0) { app.cfg.anim = v; }
    yy += 40.0;
    txt("Акцент темы", x + 16.0, yy + 20.0, 12.0, dim);
    let names = ["Aurora", "Neon", "Sunset", "Void"];
    if btn(app, "acc", names[app.cfg.accent.clamp(0, 3) as usize], x + 150.0, yy, 160.0, 32.0, false) { app.cfg.accent = (app.cfg.accent + 1) % 4; app.snd("toggle_on"); }
    yy += 44.0;
    txt("Путь config чита", x + 16.0, yy + 20.0, 12.0, dim);
    field(app, "path", x + 150.0, yy, pw - 170.0, 34.0, "авто");
    yy += 48.0;
    if btn(app, "save", "Сохранить", x + 150.0, yy, 180.0, 40.0, true) {
        app.cfg.shared = app.f_path.trim().to_string();
        app.save_all();
        app.accs = load(app.shared().join("accounts.json"));
        app.enabled = load(app.shared().join("modules.json"));
        app.say("Сохранено");
    }
    yy += 52.0;
    if cfg!(target_os = "android") {
        if btn(app, "pojav", "Открыть PojavLauncher", x + 150.0, yy, 220.0, 40.0, false) { launch(app); }
    }
}
fn info(app: &mut App) {
    let (x, y, _w, _h) = area();
    let (_, _, _, _, dim) = pal();
    let (_, a2) = accents(app.cfg.accent);
    txt("SERCH CLIENT v2.5", x, y + 30.0, 26.0, a2);
    let lines = ["Мощный и удобный клиент для Minecraft.", "Создан для тех, кто ценит свободу и стиль.", "", "Побеждает не тот, кто сильнее, а тот, кто умнее.", "", "Движок лаунчера: Rust + macroquad", "Сборка: GitHub Actions -> APK"];
    let mut yy = y + 60.0;
    for l in lines { txt(l, x, yy, 13.0, dim); yy += 24.0; }
    if btn(app, "tg", "Telegram", x, yy + 10.0, 120.0, 36.0, false) { open_url("https://t.me/"); }
}
fn launch(app: &mut App) {
    app.snd("launch");
    if cfg!(target_os = "android") {
        if open_pojav() { app.say("Открываю PojavLauncher"); } else { app.say("PojavLauncher не найден"); app.snd("error"); }
    } else { app.say("На ПК: используй desktop-сборку"); }
}

#[cfg(target_os = "android")]
fn android_dir() -> PathBuf {
    unsafe {
        let ctx = ndk_context::android_context();
        if ctx.context().is_null() { return PathBuf::from("/data/local/tmp"); }
        if let Ok(vm) = jni::JavaVM::from_raw(ctx.vm().cast()) {
if let Ok(mut env) = vm.attach_current_thread() {
                let context = jni::objects::JObject::from_raw(ctx.context().cast());
                if let Ok(f) = env.call_method(&context, "getFilesDir", "()Ljava/io/File;", &[]) {
                    if let Ok(f) = f.l() {
                        if let Ok(p) = env.call_method(&f, "getAbsolutePath", "()Ljava/lang/String;", &[]) {
                            if let Ok(s) = p.l() {
                                let js: jni::objects::JString = s.into();
                                let gs = env.get_string(&js); if let Ok(r) = gs { let p: String = r.into(); return PathBuf::from(p); }
                            }
                        }
                    }
                }
            }
        }
    }
    PathBuf::from("/data/local/tmp")
}
#[cfg(not(target_os = "android"))]
fn android_dir() -> PathBuf { home() }

#[cfg(target_os = "android")]
fn open_pojav() -> bool {
    unsafe {
        let ctx = ndk_context::android_context();
        let vm = match jni::JavaVM::from_raw(ctx.vm().cast()) { Ok(v) => v, Err(_) => return false };
        let mut env = match vm.attach_current_thread() { Ok(e) => e, Err(_) => return false };
        let context = jni::objects::JObject::from_raw(ctx.context().cast());
        let pm = match env.call_method(&context, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[]) { Ok(v) => match v.l() { Ok(o) => o, Err(_) => return false }, Err(_) => return false };
        let pkg = match env.new_string("net.kdt.pojavlaunch") { Ok(s) => s, Err(_) => return false };
        let intent = match env.call_method(&pm, "getLaunchIntentForPackage", "(Ljava/lang/String;)Landroid/content/Intent;", &[(&pkg).into()]) { Ok(v) => match v.l() { Ok(o) => o, Err(_) => return false }, Err(_) => return false };
        if intent.is_null() { return false; }
        env.call_method(&context, "startActivity", "(Landroid/content/Intent;)V", &[(&intent).into()]).is_ok()
    }
}
#[cfg(not(target_os = "android"))]
fn open_pojav() -> bool { false }

#[cfg(target_os = "android")]
fn open_url(url: &str) {
    unsafe {
        let ctx = ndk_context::android_context();
        if let Ok(vm) = jni::JavaVM::from_raw(ctx.vm().cast()) {
            if let Ok(mut env) = vm.attach_current_thread() {
                let context = jni::objects::JObject::from_raw(ctx.context().cast());
                if let (Ok(ic), Ok(uc)) = (env.find_class("android/content/Intent"), env.find_class("android/net/Uri")) {
                    if let Ok(u) = env.new_string(url) {
                        if let Ok(uri) = env.call_static_method(uc, "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[(&u).into()]) {
                            if let Ok(uo) = uri.l() {
                                if let Ok(act) = env.new_string("android.intent.action.VIEW") {
                                    if let Ok(i) = env.new_object(ic, "(Ljava/lang/String;Landroid/net/Uri;)V", &[(&act).into(), (&uo).into()]) {
                                        let _ = env.call_method(&context, "startActivity", "(Landroid/content/Intent;)V", &[(&i).into()]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
#[cfg(not(target_os = "android"))]
fn open_url(url: &str) { let _ = std::process::Command::new("xdg-open").arg(url).spawn(); }

fn window_conf() -> Conf {
    Conf { window_title: "Serch Client".to_string(), window_width: 1280, window_height: 800, fullscreen: false, high_dpi: false, sample_count: 4, ..Default::default() }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;
    loop {
        app.dt = get_frame_time().clamp(0.001, 0.1);
        app.t += app.dt;
        app.kb = false;
        let (bg, _, _, _, _) = pal();
        let (a1, a2) = accents(app.cfg.accent);
        clear_background(bg);
        glow(screen_width() * 0.9, screen_height() * 0.1, 300.0, ca(a1, 0.25));
        glow(screen_width() * 0.1, screen_height() * 0.95, 260.0, ca(a2, 0.18));
        header(&mut app);
        nav(&mut app);
        match app.tab { 0 => tab_home(&mut app), 1 => modules(&mut app), 2 => accounts(&mut app), 3 => settings(&mut app), _ => info(&mut app) }
        let h = screen_height();
        let nh = if narrow() { 62.0 } else { 0.0 };
        let sy = h - 40.0 - nh;
        let (sb, _, _, _, _) = pal();
        draw_rectangle(0.0, sy, screen_width(), 40.0, sb);
        draw_circle(18.0, sy + 20.0, 4.0, c(0.13, 0.77, 0.37));
        txt("Готов к игре", 28.0, sy + 24.0, 10.0, c(0.13, 0.77, 0.37));
        if btn(&mut app, "stl", "Запустить игру", screen_width() - 172.0, sy + 4.0, 160.0, 32.0, true) { launch(&mut app); }
        if app.toast_t > 0.0 {
            app.toast_t -= app.dt;
            let a = (app.toast_t / 2.5).clamp(0.0, 1.0);
            let wtx = tw(&app.toast, 13.0) + 40.0;
            let x = (screen_width() - wtx) / 2.0;
            let y = h - 110.0 - nh;
            rr(x, y, wtx, 40.0, 12.0, ca(a2, a));
            rr(x + 1.0, y + 1.0, wtx - 2.0, 38.0, 11.0, ca(c(0.09, 0.06, 0.14), a));
            txt(&app.toast, x + 20.0, y + 26.0, 13.0, ca(c(0.93, 0.914, 0.996), a));
        }
        next_frame().await;
    }
                       }
