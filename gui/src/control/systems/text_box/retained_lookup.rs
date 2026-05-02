pub(super) fn retained_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}
