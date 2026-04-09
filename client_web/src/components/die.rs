use leptos::prelude::*;

/// (cx, cy) positions for dots on a 48×48 die face.
fn dot_positions(value: u8) -> &'static [(&'static str, &'static str)] {
    match value {
        1 => &[("24", "24")],
        2 => &[("35", "13"), ("13", "35")],
        3 => &[("35", "13"), ("24", "24"), ("13", "35")],
        4 => &[("13", "13"), ("35", "13"), ("13", "35"), ("35", "35")],
        5 => &[("13", "13"), ("35", "13"), ("24", "24"), ("13", "35"), ("35", "35")],
        6 => &[("13", "13"), ("35", "13"), ("13", "24"), ("35", "24"), ("13", "35"), ("35", "35")],
        _ => &[],
    }
}

/// A single die face rendered as SVG.
/// `value` 1–6 shows dots; 0 shows an empty face (not-yet-rolled).
/// `used` dims the die.
/// `is_double` applies a golden glow (both dice same value).
#[component]
pub fn Die(
    value: u8,
    used: bool,
    #[prop(default = false)] is_double: bool,
) -> impl IntoView {
    let mut cls = if used {
        "die-face die-used".to_string()
    } else {
        "die-face".to_string()
    };
    if is_double && !used {
        cls.push_str(" die-double");
    }
    let dots: Vec<AnyView> = dot_positions(value)
        .iter()
        .map(|&(cx, cy)| view! { <circle cx=cx cy=cy r="4.5" /> }.into_any())
        .collect();
    view! {
        <svg class=cls width="48" height="48" viewBox="0 0 48 48">
            <rect x="1.5" y="1.5" width="45" height="45" rx="7" ry="7" />
            {dots}
        </svg>
    }
}
