use crate::account::Watch;
use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, queue};
use kodama_term::outfit::{Item, Outfit};
use kodama_term::{render, Branch, Charset, FrameInput, Pose};
use signal_hook::consts::{SIGINT, SIGTERM};
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColorMode {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

fn color_mode() -> ColorMode {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    color_mode_for(
        &env("TERM"),
        &env("COLORTERM"),
        std::env::var_os("NO_COLOR").is_some(),
    )
}

fn color_mode_for(term: &str, colorterm: &str, no_color: bool) -> ColorMode {
    let (term, colorterm) = (term.to_ascii_lowercase(), colorterm.to_ascii_lowercase());
    if no_color || term == "dumb" || term.is_empty() {
        ColorMode::None
    } else if colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || term.contains("direct")
        || term.contains("truecolor")
    {
        ColorMode::TrueColor
    } else if term.contains("256color") {
        ColorMode::Ansi256
    } else {
        ColorMode::Ansi16
    }
}

fn charset() -> Charset {
    let loc = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default();
    if loc.to_uppercase().contains("UTF") {
        Charset::Unicode
    } else {
        Charset::Ascii
    }
}

/// 进入时接管终端,Drop 与 panic 钩子都会恢复。
struct TermGuard;

impl TermGuard {
    fn enter() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut o = std::io::stdout();
        queue!(o, EnterAlternateScreen, cursor::Hide)?;
        o.flush()?;
        Ok(Self)
    }
}

fn restore() {
    let mut o = std::io::stdout();
    let _ = queue!(o, ResetColor, cursor::Show, LeaveAlternateScreen);
    let _ = o.flush();
    let _ = terminal::disable_raw_mode();
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore();
    }
}

/// 本地预览依次展示的外形：幼体、三种少年、以及各成年分支（按 g 切换）。
const FORMS: [&[Branch]; 8] = [
    &[],
    &[Branch::Curious],
    &[Branch::Gentle],
    &[Branch::Steady],
    &[Branch::Curious, Branch::Sage],
    &[Branch::Gentle, Branch::Guardian],
    &[Branch::Steady, Branch::Wanderer],
    &[Branch::Curious, Branch::Radiant],
];

/// 半格像素画的最低高度：放得下幼体；更矮或不支持 UTF-8、至少 256 色时退回字符画。
const BLOCKS_MIN_ROWS: u16 = 10;

/// 要画的个体：本地预览是固定的演示个体，账号模式来自服务的投影。
struct Look {
    species: u8,
    genes: [u8; 8],
    lineage: Vec<Branch>,
    outfit: Outfit,
    rarity: u8,
}

/// 本地预览依次试穿的装扮（按 u 切换）：不穿、草地一套、夜空一套。
fn demo_outfit(n: usize) -> Outfit {
    match n % 3 {
        1 => Outfit {
            head: Some(Item::FlowerCrown),
            face: Some(Item::RoundGlasses),
            body: Some(Item::KnitScarf),
            background: Some(Item::Meadow),
            ..Outfit::default()
        },
        2 => Outfit {
            head: Some(Item::StarHat),
            body: Some(Item::BowTie),
            back: Some(Item::HeroCape),
            background: Some(Item::NightSky),
            ..Outfit::default()
        },
        _ => Outfit::default(),
    }
}

fn preview(species: u8, form: usize, outfit: usize) -> Look {
    Look {
        species,
        genes: [37, 0, 0, 0, 0, 0, 0, 0],
        lineage: FORMS[form % FORMS.len()].to_vec(),
        outfit: demo_outfit(outfit),
        rarity: 0,
    }
}

fn input(look: Look, w: u16, h: u16, t: u64, pose: Pose, cs: Charset) -> FrameInput {
    let lineage = look.lineage;
    FrameInput {
        species: look.species,
        genes: look.genes,
        pose,
        charset: cs,
        w,
        h,
        t,
        stage: lineage.len() as u8,
        lineage,
        outfit: look.outfit,
        rarity: look.rarity,
        companion: None,
    }
}

fn form_label(form: usize) -> String {
    let lineage = FORMS[form % FORMS.len()];
    if lineage.is_empty() {
        return "baby".into();
    }
    let names: Vec<String> = lineage
        .iter()
        .map(|b| {
            serde_json::to_value(b)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default()
        })
        .collect();
    names.join(" > ")
}

/// 非 TTY:输出一行可读 JSON 状态后退出,不含任何控制码。
fn non_tty() -> i32 {
    let f = render(&input(
        preview(1, 0, 0),
        24,
        4,
        0,
        Pose::Idle,
        Charset::Ascii,
    ));
    println!("{}", serde_json::json!({ "pose": "idle", "frame": f.rows }));
    0
}

/// `account` 为 true 时显示账号宠物（需要先 `kodama login`），按键执行真实的动作。
pub fn run(account: bool) -> i32 {
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        // 账号模式在非 TTY 下等同 status：一行 JSON
        return if account {
            crate::account::run("status", &[])
        } else {
            non_tty()
        };
    }
    // 没有登录时在进入终端模式之前就失败，错误照常输出
    let mut watch = match account.then(Watch::start).transpose() {
        Ok(w) => w,
        Err(f) => {
            println!("{}", f.to_json());
            return 1;
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    for sig in [SIGINT, SIGTERM] {
        if signal_hook::flag::register(sig, stop.clone()).is_err() {
            eprintln!("cannot register signal handler");
            return 1;
        }
    }
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        prev(info);
    }));

    let guard = match TermGuard::enter() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("cannot enter terminal mode: {e}");
            return 1;
        }
    };
    let (mode, glyphs) = (color_mode(), charset());
    // 真彩色使用原 RGB；256 色只是同一像素网格的降色回退。
    let blocks =
        matches!(mode, ColorMode::TrueColor | ColorMode::Ansi256) && glyphs == Charset::Unicode;
    let mut pose = Pose::Idle;
    let mut species = 1u8;
    let mut form = 0usize;
    let mut outfit = 0usize;
    let mut t = 0u64;
    let mut out = std::io::stdout();

    while !stop.load(Ordering::Relaxed) {
        let (w, h) = terminal::size().unwrap_or((40, 10));
        let rows = h.saturating_sub(1);
        let cs = if blocks && rows >= BLOCKS_MIN_ROWS {
            Charset::Blocks
        } else {
            glyphs
        };
        let (look, pose_now) = match &mut watch {
            Some(wt) => {
                wt.tick(t);
                let look = match &wt.view {
                    Some(v) => Look {
                        species: v.species,
                        genes: v.genes,
                        lineage: v.lineage.clone(),
                        outfit: v.outfit.clone(),
                        rarity: v.rarity,
                    },
                    None => preview(1, 0, 0),
                };
                (look, wt.pose(t))
            }
            None => (preview(species, form, outfit), pose),
        };
        let f = render(&input(look, w, rows, t, pose_now, cs));
        let _ = queue!(out, cursor::MoveTo(0, 0));
        for (y, row) in f.rows.iter().enumerate() {
            let _ = queue!(out, cursor::MoveTo(0, y as u16));
            for (x, ch) in row.chars().enumerate() {
                let c = f.colors[y][x];
                match (mode, c) {
                    (ColorMode::None, _) | (_, 0) => {
                        let _ = queue!(out, ResetColor, Print(ch));
                    }
                    (ColorMode::Ansi256, c) => {
                        let _ = queue!(out, ResetColor, SetForegroundColor(Color::AnsiValue(c)));
                        if f.bg[y][x] != 0 {
                            let _ = queue!(out, SetBackgroundColor(Color::AnsiValue(f.bg[y][x])));
                        }
                        let _ = queue!(out, Print(ch));
                    }
                    (ColorMode::TrueColor, c) => {
                        let _ = queue!(out, ResetColor);
                        if let Some(rgb) = f.fg_rgb.get(y).and_then(|r| r.get(x)).copied().flatten()
                        {
                            let _ = queue!(
                                out,
                                SetForegroundColor(Color::Rgb {
                                    r: rgb[0],
                                    g: rgb[1],
                                    b: rgb[2]
                                })
                            );
                        } else {
                            let _ = queue!(out, SetForegroundColor(Color::AnsiValue(c)));
                        }
                        if let Some(rgb) = f.bg_rgb.get(y).and_then(|r| r.get(x)).copied().flatten()
                        {
                            let _ = queue!(
                                out,
                                SetBackgroundColor(Color::Rgb {
                                    r: rgb[0],
                                    g: rgb[1],
                                    b: rgb[2]
                                })
                            );
                        }
                        let _ = queue!(out, Print(ch));
                    }
                    (ColorMode::Ansi16, c) => {
                        let _ = queue!(
                            out,
                            SetForegroundColor(Color::AnsiValue(1 + c % 6)),
                            Print(ch)
                        );
                    }
                }
            }
        }
        let status = match &watch {
            Some(wt) => wt.status(),
            None => format!(
                "q quit  i idle o look e eat p play s sleep ! scare k sick v evolve x end  n species  g form [{}]  u outfit",
                form_label(form)
            ),
        };
        let display = match (mode, cs) {
            (ColorMode::TrueColor, Charset::Blocks) => "RGB pixels",
            (ColorMode::Ansi256, Charset::Blocks) => "256-color fallback",
            (ColorMode::None, _) => "no-color glyph fallback",
            _ => "glyph fallback",
        };
        let crop = if cs == Charset::Blocks && (w < 56 || rows < 18) {
            " (cropped region)"
        } else {
            ""
        };
        let status: String = format!("{display}{crop} | {status}")
            .chars()
            .take(w as usize)
            .collect();
        let _ = queue!(
            out,
            cursor::MoveTo(0, h.saturating_sub(1)),
            ResetColor,
            Print(status)
        );
        let _ = out.flush();

        if poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(k)) = read() {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        // 账号模式：按键是真实的动作，结果由服务裁决
                        KeyCode::Char(c) if watch.is_some() => {
                            let action = match c {
                                'f' => Some("feed"),
                                'p' => Some("play"),
                                'r' => Some("rest"),
                                'h' => Some("heal"),
                                _ => None,
                            };
                            if let (Some(a), Some(wt)) = (action, watch.as_mut()) {
                                wt.act(a);
                            }
                        }
                        KeyCode::Char('s') => pose = Pose::Sleep,
                        KeyCode::Char('i') => pose = Pose::Idle,
                        KeyCode::Char('k') => pose = Pose::Sick,
                        KeyCode::Char('o') => pose = Pose::Observe,
                        KeyCode::Char('e') => pose = Pose::Eat,
                        KeyCode::Char('p') => pose = Pose::Play,
                        KeyCode::Char('!') => pose = Pose::Startled,
                        KeyCode::Char('v') => pose = Pose::Evolve,
                        KeyCode::Char('x') => pose = Pose::End,
                        KeyCode::Char('n') => {
                            // 切换物种,便于逐个检查形象。
                            species = (species + 1) % kodama_term::SPECIES_COUNT;
                        }
                        // 切换进化外形：幼体 → 少年三分支 → 成年各分支
                        KeyCode::Char('g') => form = (form + 1) % FORMS.len(),
                        // 试穿装扮：不穿 → 草地一套 → 夜空一套
                        KeyCode::Char('u') => outfit += 1,
                        _ => {}
                    }
                }
            }
            // Resize 事件下一轮循环按新尺寸重绘;重绘前清屏避免残影。
            let _ = queue!(out, terminal::Clear(terminal::ClearType::All));
        }
        t += 1;
    }
    drop(guard);
    0
}

#[cfg(test)]
mod color_tests {
    use super::*;

    #[test]
    fn truecolor_is_not_demoted_to_ansi256_and_no_color_takes_priority() {
        assert_eq!(
            color_mode_for("xterm-256color", "truecolor", false),
            ColorMode::TrueColor
        );
        assert_eq!(
            color_mode_for("xterm", "24bit", false),
            ColorMode::TrueColor
        );
        assert_eq!(
            color_mode_for("xterm-direct", "", false),
            ColorMode::TrueColor
        );
        assert_eq!(
            color_mode_for("xterm-256color", "", false),
            ColorMode::Ansi256
        );
        assert_eq!(color_mode_for("xterm", "", false), ColorMode::Ansi16);
        assert_eq!(
            color_mode_for("xterm-256color", "truecolor", true),
            ColorMode::None
        );
        assert_eq!(color_mode_for("dumb", "truecolor", false), ColorMode::None);
    }
}
