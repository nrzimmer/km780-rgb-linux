//! Profiles saved **in a file of ours, not on the keyboard** -- item 4 of
//! `docs/29`.
//!
//! The keyboard already has three modes, and they are its own: the M1/M2/M3
//! keys switch between them and there are only three. A profile here is
//! another thing -- as many as the user wants, in `~/.config/km780/profiles/`,
//! and loading one writes nothing: it lands in the edit buffer and waits for
//! `Apply`, like everything else in the app.
//!
//! ## What a profile carries, and what it does not
//!
//! It carries lighting: the colors of the 144 slots, the brightness and the
//! effect.
//!
//! **It does not carry the key assignment table.** That is not an oversight. A
//! remap goes to all three modes and survives a reboot (`docs/28`); loading a
//! profile called "red" and finding out it also changed the function of eight
//! keys in every profile on the keyboard would be exactly the kind of surprise
//! this app exists to avoid. The `format` field leaves the door open for a
//! future version that asks for this explicitly.

use std::path::PathBuf;

use crate::slots::NSLOTS;

/// File format version. Goes up when the content changes shape.
const FORMAT: u64 = 1;

pub struct Profile {
    pub colors: Vec<[u8; 3]>,
    pub brightness: u8,
    pub effect: u8,
}

fn folder() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("km780").join("profiles"))
}

pub fn path() -> String {
    folder().map(|p| p.display().to_string()).unwrap_or_default()
}

/// Safe file name: only what will not turn into directory traversal or a shell
/// surprise. A name that is empty after cleaning is refused.
fn sanitize(name: &str) -> Option<String> {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn list() -> Vec<String> {
    let Some(p) = folder() else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(&p) else { return Vec::new() };
    let mut v: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    v.sort();
    v
}

pub fn save(name: &str, p: &Profile) -> Result<String, String> {
    let name = sanitize(name).ok_or("empty profile name")?;
    let dir = folder().ok_or("missing HOME or XDG_CONFIG_HOME")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let colors: Vec<serde_json::Value> = p
        .colors
        .iter()
        .map(|c| serde_json::Value::from(format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])))
        .collect();
    let mut o = serde_json::Map::new();
    o.insert("format".into(), FORMAT.into());
    o.insert("name".into(), name.clone().into());
    o.insert("brightness".into(), p.brightness.into());
    o.insert("effect".into(), p.effect.into());
    o.insert("colors".into(), serde_json::Value::Array(colors));

    let file_path = dir.join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(o))
        .map_err(|e| e.to_string())?;
    std::fs::write(&file_path, text).map_err(|e| format!("{}: {e}", file_path.display()))?;
    Ok(file_path.display().to_string())
}

pub fn load(name: &str) -> Result<Profile, String> {
    let name = sanitize(name).ok_or("empty profile name")?;
    let file_path = folder()
        .ok_or("missing HOME or XDG_CONFIG_HOME")?
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("{}: {e}", file_path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    // Future format: refusing beats reading it halfway and saying it worked.
    // A profile read wrong only shows up after Apply, on the keyboard.
    match v.get("format").and_then(|f| f.as_u64()) {
        Some(f) if f <= FORMAT => {}
        Some(f) => return Err(format!("profile is format {f}; this version reads up to {FORMAT}")),
        None => return Err("file has no 'format' field \u{2014} not a km780 profile".into()),
    }

    let arr = v
        .get("colors")
        .and_then(|c| c.as_array())
        .ok_or("profile has no 'colors' list")?;
    if arr.len() != NSLOTS {
        return Err(format!("profile has {} colors, expected {NSLOTS}", arr.len()));
    }
    let mut colors = Vec::with_capacity(NSLOTS);
    for (i, c) in arr.iter().enumerate() {
        let s = c.as_str().ok_or_else(|| format!("color {i} is not a string"))?;
        let n = u32::from_str_radix(s, 16)
            .map_err(|_| format!("color {i} ({s:?}) is not RRGGBB"))?;
        colors.push([(n >> 16) as u8, (n >> 8) as u8, n as u8]);
    }

    let byte = |k: &str| -> Result<u8, String> {
        v.get(k)
            .and_then(|x| x.as_u64())
            .filter(|&x| x <= 255)
            .map(|x| x as u8)
            .ok_or_else(|| format!("profile has no valid '{k}'"))
    };
    let brightness = byte("brightness")?;
    if brightness > 4 {
        return Err(format!("profile brightness {brightness} is out of 0..4"));
    }
    Ok(Profile { colors, brightness, effect: byte("effect")? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_cannot_escape_the_folder() {
        assert_eq!(sanitize("../../etc/passwd").as_deref(), Some("etc-passwd"));
        assert_eq!(sanitize("cold night").as_deref(), Some("cold-night"));
        assert_eq!(sanitize("   ").as_deref(), None);
        assert_eq!(sanitize("///").as_deref(), None);
    }
}
