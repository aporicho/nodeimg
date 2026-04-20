pub const PART_SEPARATOR: &str = "::";

pub fn part_id(root: &str, part: &str) -> String {
    format!("{root}{PART_SEPARATOR}{part}")
}

pub fn owner_id(id: &str) -> &str {
    id.split(PART_SEPARATOR).next().unwrap_or(id)
}

#[derive(Debug, Clone, Copy)]
pub struct Anatomy<'a> {
    root: &'a str,
}

impl<'a> Anatomy<'a> {
    pub fn new(root: &'a str) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &'a str {
        self.root
    }

    pub fn part(&self, part: &str) -> String {
        part_id(self.root, part)
    }

    pub fn label(&self) -> String {
        self.part("label")
    }

    pub fn field(&self) -> String {
        self.part("field")
    }

    pub fn content(&self) -> String {
        self.part("content")
    }

    pub fn track(&self) -> String {
        self.part("track")
    }

    pub fn thumb(&self) -> String {
        self.part("thumb")
    }

    pub fn titlebar(&self) -> String {
        self.part("titlebar")
    }

    pub fn title(&self) -> String {
        self.part("title")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_id_builds_stable_name() {
        assert_eq!(part_id("slider", "track"), "slider::track");
    }

    #[test]
    fn owner_id_strips_part_suffix() {
        assert_eq!(owner_id("text_input::field"), "text_input");
    }

    #[test]
    fn owner_id_keeps_plain_root() {
        assert_eq!(owner_id("button"), "button");
    }
}
