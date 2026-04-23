use super::IconRegistry;

const PLUS: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M12 5V19M5 12H19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

const CLOSE: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M6 6L18 18M18 6L6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

const CHEVRON_DOWN: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M6 9L12 15L18 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

const CHEVRON_RIGHT: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M9 6L15 12L9 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

const PLAY: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M8 5L19 12L8 19V5Z" fill="currentColor"/>
</svg>
"#;

const SEARCH: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M11 19C15.4183 19 19 15.4183 19 11C19 6.58172 15.4183 3 11 3C6.58172 3 3 6.58172 3 11C3 15.4183 6.58172 19 11 19Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M21 21L16.65 16.65" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

const SETTINGS: &str = r#"
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <path d="M12 15.5C13.933 15.5 15.5 13.933 15.5 12C15.5 10.067 13.933 8.5 12 8.5C10.067 8.5 8.5 10.067 8.5 12C8.5 13.933 10.067 15.5 12 15.5Z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M19.4 15A1.7 1.7 0 0 0 19.74 16.88L19.8 16.94A2.06 2.06 0 0 1 16.94 19.8L16.88 19.74A1.7 1.7 0 0 0 15 19.4A1.7 1.7 0 0 0 14 20.95V21.12A2.06 2.06 0 0 1 9.88 21.12V20.95A1.7 1.7 0 0 0 8.8 19.4A1.7 1.7 0 0 0 6.92 19.74L6.86 19.8A2.06 2.06 0 0 1 4 16.94L4.06 16.88A1.7 1.7 0 0 0 4.4 15A1.7 1.7 0 0 0 2.85 14H2.68A2.06 2.06 0 0 1 2.68 9.88H2.85A1.7 1.7 0 0 0 4.4 8.8A1.7 1.7 0 0 0 4.06 6.92L4 6.86A2.06 2.06 0 0 1 6.86 4L6.92 4.06A1.7 1.7 0 0 0 8.8 4.4H8.82A1.7 1.7 0 0 0 9.88 2.85V2.68A2.06 2.06 0 0 1 14 2.68V2.85A1.7 1.7 0 0 0 15.08 4.4A1.7 1.7 0 0 0 16.96 4.06L17.02 4A2.06 2.06 0 0 1 19.88 6.86L19.82 6.92A1.7 1.7 0 0 0 19.48 8.8V8.82A1.7 1.7 0 0 0 21.03 9.88H21.2A2.06 2.06 0 0 1 21.2 14H21.03A1.7 1.7 0 0 0 19.4 15Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"#;

pub(crate) fn register_builtin_icons(registry: &mut IconRegistry) {
    registry.register_svg("plus", PLUS.as_bytes());
    registry.register_svg("close", CLOSE.as_bytes());
    registry.register_svg("chevron_down", CHEVRON_DOWN.as_bytes());
    registry.register_svg("chevron_right", CHEVRON_RIGHT.as_bytes());
    registry.register_svg("play", PLAY.as_bytes());
    registry.register_svg("search", SEARCH.as_bytes());
    registry.register_svg("settings", SETTINGS.as_bytes());
}
