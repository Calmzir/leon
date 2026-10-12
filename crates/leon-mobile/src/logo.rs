//! A project's logo, found on the other computer the way the desktop finds
//! it.
//!
//! Nothing about logos travels with what a host shares, so the phone looks
//! for itself, with the desktop's own detection (`leon_remote::icon`): the
//! same rules, the same script, the same caps, run through the session's
//! command primitive instead of a local process. The choice is the desktop's
//! too: an image file of the repository first; without one, the avatar of the
//! repository's owner when the origin is on GitHub; without that, nothing,
//! and the caller shows the folder glyph.
//!
//! What a phone can draw is narrower than what the desktop accepts. PNG, JPEG
//! and WebP go as they are and SVG as its source. A Windows icon is a
//! container: when one of its entries is a PNG the largest such entry is
//! handed over; one that holds only bitmaps counts as no image (and the
//! avatar is offered instead). The avatar is a URL the caller fetches: the
//! one request that leaves for a third party, as on the desktop.

use base64::Engine;
use leon_core::icon::{content_hash, IconFormat as CoreFormat, IconImage};
use leon_core::{Machine, MachineId, MachineKind};
use leon_link::client::Client;
use leon_remote::command::{CommandSpec, SshOptions};
use leon_remote::runner::{Output, RunError, Runner};
use leon_wire::ExecSpec;

use crate::error::MobileError;

/// How long one command of the scan may take.
const SCAN_TIMEOUT_MS: u32 = 30_000;

/// The image formats handed to the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum LogoFormat {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
    /// WebP.
    Webp,
    /// SVG.
    Svg,
}

/// An image file of the project.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LogoImage {
    /// What `data` is.
    pub format: LogoFormat,
    /// The image: base64 of the file for PNG, JPEG and WebP, the source text
    /// for SVG.
    pub data: String,
    /// A fingerprint of the image: the same image has the same one.
    pub hash: String,
    /// The rule and the file it came from, `nextjs-app:app/icon.png`.
    pub source: String,
}

/// What stands for a project. With neither field, the folder glyph does.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct ProjectLogo {
    /// An image found in the repository.
    pub image: Option<LogoImage>,
    /// Where the avatar of the repository's owner is, offered only when no
    /// image was found and the origin is on GitHub.
    pub avatar_url: Option<String>,
}

/// Runs the commands of the scan on the other computer.
struct OverSession(Client);

impl Runner for OverSession {
    async fn run(&self, spec: &CommandSpec) -> Result<Output, RunError> {
        let wire = ExecSpec {
            program: spec.program.clone(),
            args: spec.args.clone(),
            env: spec.env.clone(),
            cwd: spec.cwd.clone(),
            stdin: None,
        };
        match self.0.exec(wire, Some(SCAN_TIMEOUT_MS)).await {
            Ok(out) => Ok(Output {
                status: out.status,
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            }),
            Err(error) => Err(RunError::Spawn {
                program: spec.program.clone(),
                source: std::io::Error::other(error.to_string()),
            }),
        }
    }
}

/// Looks for the logo of the project rooted at `root` on the computer at the
/// other end of `client`.
pub(crate) async fn find(client: Client, root: &str) -> Result<ProjectLogo, MobileError> {
    // The session already ends on the right computer: to the detection it is
    // the machine the commands start on.
    let here = Machine {
        id: MachineId::local(),
        name: String::new(),
        kind: MachineKind::Local,
    };
    let runner = OverSession(client.clone());
    let detection = leon_remote::icon::detect(&runner, &here, &SshOptions::default(), root)
        .await
        .map_err(|error| match client.state() {
            leon_link::client::ConnState::Online => MobileError::Host {
                message: error.to_string(),
            },
            _ => MobileError::Disconnected,
        })?;
    let image = detection
        .icon
        .and_then(|found| drawable(&found.image).map(|image| (found.rule, found.path, image)))
        .map(|(rule, path, (format, data, hash))| LogoImage {
            format,
            data,
            hash,
            source: format!("{rule}:{path}"),
        });
    let avatar_url = match image {
        Some(_) => None,
        None => detection.remote.and_then(|remote| remote.avatar_url()),
    };
    Ok(ProjectLogo { image, avatar_url })
}

/// `image` as a phone can draw it: its format, its data and its fingerprint.
fn drawable(image: &IconImage) -> Option<(LogoFormat, String, String)> {
    let encode = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let hash = content_hash(&image.bytes);
    match image.format {
        CoreFormat::Png => Some((LogoFormat::Png, encode(&image.bytes), hash)),
        CoreFormat::Jpeg => Some((LogoFormat::Jpeg, encode(&image.bytes), hash)),
        CoreFormat::Webp => Some((LogoFormat::Webp, encode(&image.bytes), hash)),
        CoreFormat::Svg => String::from_utf8(image.bytes.clone())
            .ok()
            .map(|source| (LogoFormat::Svg, source, hash)),
        CoreFormat::Ico => {
            let png = largest_png(&image.bytes)?;
            Some((LogoFormat::Png, encode(png), content_hash(png)))
        }
    }
}

/// The largest entry of a Windows icon that is a PNG, when it has one.
fn largest_png(ico: &[u8]) -> Option<&[u8]> {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    let word = |at: usize| Some(u16::from_le_bytes(ico.get(at..at + 2)?.try_into().ok()?));
    let long = |at: usize| Some(u32::from_le_bytes(ico.get(at..at + 4)?.try_into().ok()?) as usize);
    if word(0)? != 0 || word(2)? != 1 {
        return None;
    }
    let mut best: Option<(u32, &[u8])> = None;
    for entry in 0..usize::from(word(4)?) {
        let at = 6 + entry * 16;
        // A side of zero stands for 256.
        let side = |byte: u8| if byte == 0 { 256 } else { u32::from(byte) };
        let (Some(&width), Some(&height)) = (ico.get(at), ico.get(at + 1)) else {
            break;
        };
        let (Some(size), Some(offset)) = (long(at + 8), long(at + 12)) else {
            break;
        };
        let Some(bytes) = offset
            .checked_add(size)
            .and_then(|end| ico.get(offset..end))
        else {
            continue;
        };
        let area = side(width) * side(height);
        if bytes.starts_with(PNG) && best.is_none_or(|(most, _)| area > most) {
            best = Some((area, bytes));
        }
    }
    best.map(|(_, bytes)| bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-pixel PNG.
    pub(crate) const PIXEL: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// A Windows icon with these entries: a side and the entry's bytes.
    fn ico(entries: &[(u8, &[u8])]) -> Vec<u8> {
        let mut out = vec![0, 0, 1, 0];
        out.extend((entries.len() as u16).to_le_bytes());
        let mut offset = 6 + entries.len() * 16;
        for (side, bytes) in entries {
            out.extend([*side, *side, 0, 0, 1, 0, 32, 0]);
            out.extend((bytes.len() as u32).to_le_bytes());
            out.extend((offset as u32).to_le_bytes());
            offset += bytes.len();
        }
        for (_, bytes) in entries {
            out.extend(*bytes);
        }
        out
    }

    #[test]
    fn the_largest_png_of_a_windows_icon_is_taken() {
        let mut big = PIXEL.to_vec();
        big.extend(b"big");
        let bitmap = [40u8, 0, 0, 0, 1, 2, 3];
        let file = ico(&[(16, PIXEL), (0, &big), (48, &bitmap)]);
        assert_eq!(largest_png(&file), Some(big.as_slice()));
    }

    #[test]
    fn a_windows_icon_of_bitmaps_only_has_no_png() {
        let bitmap = [40u8, 0, 0, 0, 1, 2, 3];
        assert_eq!(largest_png(&ico(&[(16, &bitmap), (32, &bitmap)])), None);
    }

    #[test]
    fn a_damaged_windows_icon_is_never_read_past_its_end() {
        let file = ico(&[(16, PIXEL)]);
        for cut in 0..file.len() {
            let _ = largest_png(&file[..cut]);
        }
        let mut lying = file.clone();
        lying[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(largest_png(&lying), None);
        assert_eq!(largest_png(b"not an icon at all"), None);
    }

    #[test]
    fn rasters_go_as_base64_and_vectors_as_their_source() {
        let png = IconImage {
            format: CoreFormat::Png,
            bytes: PIXEL.to_vec(),
        };
        let (format, data, hash) = drawable(&png).unwrap();
        assert_eq!(format, LogoFormat::Png);
        assert!(data.starts_with("iVBORw0KGgo"));
        assert_eq!(hash, content_hash(PIXEL));

        let svg = IconImage {
            format: CoreFormat::Svg,
            bytes: b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(),
        };
        let (format, data, _) = drawable(&svg).unwrap();
        assert_eq!(format, LogoFormat::Svg);
        assert!(data.starts_with("<svg"));
    }

    #[test]
    fn a_windows_icon_is_handed_over_as_its_png_or_not_at_all() {
        let with_png = IconImage {
            format: CoreFormat::Ico,
            bytes: ico(&[(32, PIXEL)]),
        };
        let (format, _, hash) = drawable(&with_png).unwrap();
        assert_eq!(format, LogoFormat::Png);
        assert_eq!(hash, content_hash(PIXEL));

        let bitmaps = IconImage {
            format: CoreFormat::Ico,
            bytes: ico(&[(16, &[40, 0, 0, 0])]),
        };
        assert_eq!(drawable(&bitmaps), None);
    }
}
