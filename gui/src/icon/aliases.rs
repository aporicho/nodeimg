pub(crate) fn canonical_id(id: &str) -> Option<&'static str> {
    match id {
        "close" => Some("xmark"),
        "chevron_down" => Some("nav-arrow-down"),
        "chevron_right" => Some("nav-arrow-right"),
        _ => None,
    }
}
