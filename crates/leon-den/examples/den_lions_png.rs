//! Draws the lions of the Den in 2.5D one by one, on nothing: the pictures
//! the phone application shows them with.
//!
//! ```text
//! cargo run -p leon-den --features den3d --example den_lions_png -- OUT_DIR
//! ```
//!
//! For each agent's colour and each of a few individuals it writes
//! `<agent>_<n>.png`: a strip of [`FRAMES`] pictures of the same lion, each
//! [`WIDE`] by [`HIGH`], in the order of [`ACTS`]. A lion is drawn twice, on
//! black and on white, and what differs between the two is how much of the
//! background shows through: so the edges are soft and the rest is clear.
//!
//! `DEN_LION_TURN` turns the lions (radians) and `DEN_LION_COUNT` sets how
//! many individuals there are of each colour.

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

use leon_den::bitmap::Bitmap;
use leon_den::iso::lion::{lion, Fit, Pose, Traits, SCALE};
use leon_den::iso::mesh::{hex, Mesh, Part};
use leon_den::iso::{Camera, Renderer, Theme};
use leon_den::pose::{Act, Facing, Look};

/// One picture of a strip.
const WIDE: u32 = 200;
const HIGH: u32 = 260;

/// What a lion does in each picture, and at which beat of it.
const ACTS: [(Act, u64); 10] = [
    (Act::Stand, 0),
    (Act::Type, 0),
    (Act::Type, 1),
    (Act::Browse, 0),
    (Act::Browse, 1),
    (Act::Think, 0),
    (Act::Think, 1),
    (Act::Stare, 0),
    (Act::Sleep, 0),
    (Act::Faint, 0),
];
/// Pictures in a strip.
const FRAMES: u32 = ACTS.len() as u32;

/// The agents and the colour of each in the dark theme.
const AGENTS: [(&str, u32); 4] = [
    ("claude", 0xD97757),
    ("codex", 0xFAFAF9),
    ("opencode", 0xF1ECEC),
    ("other", 0xA8A29E),
];

fn main() {
    let out = std::env::args().nth(1).expect("a folder to write to");
    std::fs::create_dir_all(&out).expect("the folder is made");
    let turn: f32 = std::env::var("DEN_LION_TURN")
        .ok()
        .and_then(|turn| turn.parse().ok())
        .unwrap_or(std::f32::consts::FRAC_PI_4);
    let count: u64 = std::env::var("DEN_LION_COUNT")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(6);
    let mut gpu = Renderer::start().expect("a GPU");
    let theme = Theme::from_tokens(&support::tokens(true));
    // The same box for every picture, so a lion does not jump between them:
    // room for one lying down and for one standing.
    let camera = Camera::around(
        [-0.5, 0., -0.5],
        [0.5, 1.3, 0.5],
        WIDE as f32,
        HIGH as f32,
        6.,
    );
    for (agent, colour) in AGENTS {
        for individual in 0..count {
            // Seeds far apart, so the individuals of a colour differ.
            let traits = Traits::of(individual * 7919 + u64::from(colour));
            let mut strip = vec![0u8; (WIDE * FRAMES * HIGH * 4) as usize];
            for (at, (act, beat)) in ACTS.iter().enumerate() {
                let look = Look {
                    act: *act,
                    facing: Facing::Down,
                    little: false,
                    seat: false,
                    beat: *beat,
                    bubble: None,
                };
                let mut mesh = Mesh::new(theme.edge, true);
                lion(
                    &mut mesh,
                    &theme,
                    Part::at(0., 0., 0.).turned(turn),
                    &traits,
                    hex(colour),
                    &Pose::of(&look, 0.),
                    Fit {
                        seat: 0.,
                        scale: SCALE,
                    },
                );
                let on = |gpu: &mut Renderer, ground: f32| {
                    let mut theme = theme;
                    theme.background = [ground, ground, ground];
                    gpu.draw(&mesh, &camera, &theme, (WIDE, HIGH))
                        .expect("the lion is drawn")
                };
                let (black, white) = (on(&mut gpu, 0.), on(&mut gpu, 1.));
                for y in 0..HIGH {
                    for x in 0..WIDE {
                        let from = ((y * WIDE + x) * 4) as usize;
                        let to = ((y * WIDE * FRAMES + at as u32 * WIDE + x) * 4) as usize;
                        // On white it is `c + (1 - a)`, on black `c`: both
                        // premultiplied, blue first.
                        let shows = (0..3)
                            .map(|c| i32::from(white[from + c]) - i32::from(black[from + c]))
                            .max()
                            .unwrap_or(0)
                            .clamp(0, 255);
                        let alpha = 255 - shows;
                        if alpha == 0 {
                            continue;
                        }
                        for c in 0..3 {
                            let straight = i32::from(black[from + 2 - c]) * 255 / alpha;
                            strip[to + c] = straight.clamp(0, 255) as u8;
                        }
                        strip[to + 3] = alpha as u8;
                    }
                }
            }
            let name = format!("{out}/{agent}_{individual}.png");
            let picture = Bitmap::from_rgba((WIDE * FRAMES) as i32, HIGH as i32, strip);
            std::fs::write(&name, picture.to_png()).expect("the png is written");
            println!("{name}");
        }
    }
}
