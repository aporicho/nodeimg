#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UploadStats {
    pub bytes: usize,
    pub buffer_grows: usize,
}

impl UploadStats {
    pub fn add(&mut self, other: UploadStats) {
        self.bytes += other.bytes;
        self.buffer_grows += other.buffer_grows;
    }
}
