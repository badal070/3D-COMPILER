// compiler/runtime/step_trace/color_table.rs
// ColorTable — 16-color palette shared with frontend

use serde::{Deserialize, Serialize};
use serde_json::json;

/// A single color entry in the palette
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ColorEntry {
    /// Index into the 16-color palette (0–15)
    pub index: u8,

    /// Human-readable color name
    pub name: &'static str,

    /// CSS color name or hex value for frontend
    pub css_value: &'static str,

    /// Three.js/Babylon.js hex color value (e.g., 0xFF6B00)
    pub hex_value: u32,

    /// RGB components for frontend CSS
    pub rgb: (u8, u8, u8),
}

impl ColorEntry {
    /// Get the color as a hex string (#RRGGBB format)
    pub fn hex_string(&self) -> String {
        format!("#{:06X}", self.hex_value)
    }

    /// Get the color as CSS rgb() format
    pub fn rgb_string(&self) -> String {
        let (r, g, b) = self.rgb;
        format!("rgb({}, {}, {})", r, g, b)
    }

    /// Serialize to JSON for frontend embedding
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "index": self.index,
            "name": self.name,
            "css_value": self.css_value,
            "hex_value": format!("0x{:06X}", self.hex_value),
            "rgb": [self.rgb.0, self.rgb.1, self.rgb.2],
        })
    }
}

/// Fixed 16-color palette shared between Rust backend and JavaScript frontend
/// Must be identical on both sides of the Rust/WASM boundary
pub struct ColorTable;

impl ColorTable {
    /// Get a color entry by index
    /// Panics if index > 15 (programmer error, not user error)
    pub fn get(index: u8) -> &'static ColorEntry {
        &PALETTE[index as usize]
    }

    /// Get all 16 colors
    pub fn all() -> &'static [ColorEntry] {
        &PALETTE
    }

    /// Serialize the entire palette as JSON for frontend embedding
    pub fn as_json() -> serde_json::Value {
        json!({
            "palette": PALETTE.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
            "count": 16,
        })
    }

    /// Get a color for a specific token using the registry
    pub fn for_token(
        token: &str,
        registry: &crate::step_trace::token::HighlightTokenRegistry,
    ) -> Option<&'static ColorEntry> {
        registry
            .color_for(token)
            .map(|idx| Self::get(idx))
    }

    /// Get a token string formatted with color styling for frontend
    pub fn format_with_color(text: &str, color_index: u8) -> String {
        let entry = Self::get(color_index);
        format!(
            "<span style=\"color: {}; font-weight: bold;\">{}</span>",
            entry.css_value, text
        )
    }
}

/// The 16-color palette
/// CRITICAL: Must match exactly with frontend JavaScript
/// Colors chosen for perceptual distinction and accessibility
const PALETTE: [ColorEntry; 16] = [
    // 0: Amber/Orange
    ColorEntry {
        index: 0,
        name: "amber",
        css_value: "rgb(255, 140, 0)",
        hex_value: 0xFF8C00,
        rgb: (255, 140, 0),
    },
    // 1: Teal/Cyan
    ColorEntry {
        index: 1,
        name: "teal",
        css_value: "rgb(32, 178, 170)",
        hex_value: 0x20B2AA,
        rgb: (32, 178, 170),
    },
    // 2: Rose/Pink
    ColorEntry {
        index: 2,
        name: "rose",
        css_value: "rgb(219, 112, 147)",
        hex_value: 0xDB7093,
        rgb: (219, 112, 147),
    },
    // 3: Indigo/Blue
    ColorEntry {
        index: 3,
        name: "indigo",
        css_value: "rgb(75, 0, 130)",
        hex_value: 0x4B0082,
        rgb: (75, 0, 130),
    },
    // 4: Lime/Green
    ColorEntry {
        index: 4,
        name: "lime",
        css_value: "rgb(50, 205, 50)",
        hex_value: 0x32CD32,
        rgb: (50, 205, 50),
    },
    // 5: Coral/Red-Orange
    ColorEntry {
        index: 5,
        name: "coral",
        css_value: "rgb(255, 127, 80)",
        hex_value: 0xFF7F50,
        rgb: (255, 127, 80),
    },
    // 6: Deep Sky Blue
    ColorEntry {
        index: 6,
        name: "sky",
        css_value: "rgb(0, 191, 255)",
        hex_value: 0x00BFFF,
        rgb: (0, 191, 255),
    },
    // 7: Medium Orchid
    ColorEntry {
        index: 7,
        name: "orchid",
        css_value: "rgb(186, 85, 211)",
        hex_value: 0xBA55D3,
        rgb: (186, 85, 211),
    },
    // 8: Gold/Yellow
    ColorEntry {
        index: 8,
        name: "gold",
        css_value: "rgb(255, 215, 0)",
        hex_value: 0xFFD700,
        rgb: (255, 215, 0),
    },
    // 9: Crimson/Red
    ColorEntry {
        index: 9,
        name: "crimson",
        css_value: "rgb(220, 20, 60)",
        hex_value: 0xDC143C,
        rgb: (220, 20, 60),
    },
    // 10: Turquoise
    ColorEntry {
        index: 10,
        name: "turquoise",
        css_value: "rgb(64, 224, 208)",
        hex_value: 0x40E0D0,
        rgb: (64, 224, 208),
    },
    // 11: Medium Purple
    ColorEntry {
        index: 11,
        name: "purple",
        css_value: "rgb(147, 112, 219)",
        hex_value: 0x9370DB,
        rgb: (147, 112, 219),
    },
    // 12: Forest Green
    ColorEntry {
        index: 12,
        name: "forest",
        css_value: "rgb(34, 139, 34)",
        hex_value: 0x228B22,
        rgb: (34, 139, 34),
    },
    // 13: Orange Red
    ColorEntry {
        index: 13,
        name: "orange_red",
        css_value: "rgb(255, 69, 0)",
        hex_value: 0xFF4500,
        rgb: (255, 69, 0),
    },
    // 14: Steel Blue
    ColorEntry {
        index: 14,
        name: "steel",
        css_value: "rgb(70, 130, 180)",
        hex_value: 0x4682B4,
        rgb: (70, 130, 180),
    },
    // 15: Olive Drab
    ColorEntry {
        index: 15,
        name: "olive",
        css_value: "rgb(107, 142, 35)",
        hex_value: 0x6B8E23,
        rgb: (107, 142, 35),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_size() {
        assert_eq!(ColorTable::all().len(), 16);
    }

    #[test]
    fn test_get_color() {
        let color = ColorTable::get(0);
        assert_eq!(color.index, 0);
        assert_eq!(color.name, "amber");
    }

    #[test]
    #[should_panic]
    fn test_out_of_bounds() {
        let _ = ColorTable::get(16); // Should panic
    }

    #[test]
    fn test_hex_string() {
        let color = ColorTable::get(0);
        assert_eq!(color.hex_string(), "#FF8C00");
    }

    #[test]
    fn test_rgb_string() {
        let color = ColorTable::get(0);
        assert_eq!(color.rgb_string(), "rgb(255, 140, 0)");
    }

    #[test]
    fn test_color_uniqueness() {
        let colors = ColorTable::all();
        let mut hex_values: Vec<u32> = colors.iter().map(|c| c.hex_value).collect();
        hex_values.sort();
        hex_values.dedup();

        // All hex values should be unique
        assert_eq!(hex_values.len(), 16);
    }

    #[test]
    fn test_as_json() {
        let json = ColorTable::as_json();
        assert!(json["palette"].is_array());
        assert_eq!(json["count"], 16);
    }

    #[test]
    fn test_all_indices_present() {
        let colors = ColorTable::all();
        for i in 0..16 {
            assert_eq!(colors[i].index, i as u8);
        }
    }
}
