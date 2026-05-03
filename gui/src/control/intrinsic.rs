#[derive(Clone, Debug, PartialEq)]
pub struct ControlIntrinsic {
    pub control_id: String,
    pub current_size: [f32; 2],
    pub min_size: [f32; 2],
    pub desired_size: [f32; 2],
    pub affects_parent_width: bool,
    pub affects_parent_height: bool,
}
