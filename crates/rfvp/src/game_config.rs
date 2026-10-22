//! Optional console game selection. A deliberately small TOML subset: one
//! [game] table, quoted string values, comments, no arrays or inline tables.
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use anyhow::{anyhow, bail, Result};
use crate::script::parser::Nls;

#[derive(Default)]
pub(crate) struct GameConfig {
    pub hcb: Option<String>,
    pub nls: Option<Nls>,
    pub text_patch: Option<String>,
    pub text_nls: Option<Nls>,
    pub hcb_crc32: Option<u32>,
    pub asset_aliases: Option<String>,
}

pub(crate) fn relative_path(value: &str) -> Result<()> {
    if value.is_empty() || value.contains(['\\', ':', '\0']) || value.starts_with('/')
        || value.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        bail!("expected a relative path inside the game directory: {value:?}");
    }
    Ok(())
}

impl GameConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let text = core::str::from_utf8(bytes)?.trim_start_matches('\u{feff}');
        let mut config = Self::default();
        let mut table = false;
        let mut seen: Vec<&str> = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if line == "[game]" && !table { table = true; continue; }
            if !table { bail!("game.toml line {}: expected [game]", line_number + 1); }
            let (key, value) = line.split_once('=').ok_or_else(|| anyhow!("invalid game.toml line {}", line_number + 1))?;
            let key = key.trim();
            if seen.contains(&key) { bail!("duplicate game.toml key: {key}"); }
            seen.push(key);
            let value = value.trim();
            let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')
                .ok_or_else(|| anyhow!("game.toml {key}: use a quoted string"))?;
            let tail = &value[1..];
            let end = tail.find(quote).ok_or_else(|| anyhow!("unterminated game.toml {key}"))?;
            let rest = tail[end + 1..].trim();
            if !rest.is_empty() && !rest.starts_with('#') { bail!("unexpected data after game.toml {key}"); }
            let value = &tail[..end];
            if value.chars().any(|c| c.is_control() || c == '\\') {
                bail!("game.toml {key}: escapes/control characters are unsupported; use / in paths");
            }
            match key {
                "hcb" => { relative_path(value)?; config.hcb = Some(value.to_string()); }
                "nls" => config.nls = Some(value.parse()?),
                "text_patch" => { relative_path(value)?; config.text_patch = Some(value.to_string()); }
                "text_nls" => config.text_nls = Some(value.parse()?),
                "hcb_crc32" => config.hcb_crc32 = Some(u32::from_str_radix(value, 16)?),
                "asset_aliases" => { relative_path(value)?; config.asset_aliases = Some(value.to_string()); }
                _ => bail!("unknown game.toml key: {key}"),
            }
        }
        if !table { bail!("game.toml is missing [game]"); }
        if config.text_patch.is_some() && (config.text_nls.is_none() || config.hcb_crc32.is_none() || config.hcb.is_none()) {
            bail!("text_patch requires explicit hcb, hcb_crc32 and text_nls");
        }
        if config.text_patch.is_none() && config.text_nls.is_some() {
            bail!("text_nls requires text_patch");
        }
        Ok(config)
    }
}

pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut n = i as u32;
        for _ in 0..8 { n = (n >> 1) ^ (0xedb88320u32 & 0u32.wrapping_sub(n & 1)); }
        *slot = n;
    }
    let mut crc = !0u32;
    for &byte in bytes { crc = table[((crc as u8) ^ byte) as usize] ^ (crc >> 8); }
    !crc
}
