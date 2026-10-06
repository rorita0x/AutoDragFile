use serde::Deserialize;
use std::fs;
use std::path::Path;
use toml::Table;

pub const DEFAULT_CONFIG: &str = include_str!("default_config.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub text: Text,
    pub style: Style,
    pub behavior: Behavior,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub label: String,
    pub window_title: String,
    pub drag_finished: String,
    pub timeout_reached: String,
    pub file_not_found: String,
    pub gtk_init_failed: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub foreground: String,
    pub background: String,
    pub font_weight: String,
    pub font_size: String,
    pub padding: u32,
    pub border_radius: u32,
    pub width: i32,
    pub height: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behavior {
    pub timeout_ms: u64,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Config {
    pub fn load(user_config: Option<&Path>) -> Result<Self, String> {
        let mut table: Table = DEFAULT_CONFIG
            .parse()
            .map_err(|e| format!("Built-in default config is invalid: {e}"))?;

        if let Some(path) = user_config {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
            let user: Table = content
                .parse()
                .map_err(|e| format!("Invalid TOML in {}: {e}", path.display()))?;
            merge(&mut table, user);
        }

        table
            .try_into()
            .map_err(|e| format!("Invalid config: {e}"))
    }

    pub fn css(&self) -> String {
        let s = &self.style;
        format!(
            "window.overlay {{ background: transparent; }}
            .drag-box {{
                background: {bg};
                border-radius: {radius}px;
                padding: {padding}px;
            }}
            .drag-box label {{
                color: {fg};
                font-weight: {weight};
                font-size: {size};
            }}",
            bg = s.background,
            fg = s.foreground,
            radius = s.border_radius,
            padding = s.padding,
            weight = s.font_weight,
            size = s.font_size,
        )
    }
}

fn merge(base: &mut Table, overlay: Table) {
    for (key, value) in overlay {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(base_sub)), toml::Value::Table(overlay_sub)) => {
                merge(base_sub, overlay_sub);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}
