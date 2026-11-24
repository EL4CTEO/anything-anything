//! A starting logo for a new tool, as SVG: a rounded tile in the tool's colour, with its initial and the
//! anything-anything spark. Replace it whenever you like; it's yours.

use crate::names::Meta;

fn shade((r, g, b): (u8, u8, u8), k: f32) -> String {
    let f = |c: u8| ((c as f32) * k).clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", f(r), f(g), f(b))
}

pub fn svg(meta: &Meta) -> String {
    let initial = meta.verb.chars().next().unwrap_or('a').to_ascii_uppercase();
    let light = shade(meta.rgb, 1.25);
    let dark = shade(meta.rgb, 0.6);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512" role="img" aria-label="{name}">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="{light}"/>
      <stop offset="1" stop-color="{dark}"/>
    </linearGradient>
  </defs>
  <rect x="16" y="16" width="480" height="480" rx="112" fill="url(#bg)"/>
  <rect x="16" y="16" width="480" height="480" rx="112" fill="none" stroke="#ffffff" stroke-opacity="0.18" stroke-width="4"/>
  <text x="236" y="352" text-anchor="middle" font-family="Inter, Segoe UI, Helvetica, Arial, sans-serif" font-weight="800" font-size="280" fill="#ffffff">{initial}</text>
  <path d="M392 96 L404 132 L440 144 L404 156 L392 192 L380 156 L344 144 L380 132 Z" fill="#ffffff" fill-opacity="0.92"/>
</svg>
"##,
        name = meta.name
    )
}
