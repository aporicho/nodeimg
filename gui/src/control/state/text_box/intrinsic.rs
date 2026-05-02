use super::diagnostics::log_control_intrinsic;
use super::runtime::TextBoxRuntime;
use super::store::TextBoxStore;
use crate::control::ControlIntrinsic;

impl TextBoxStore {
    pub(crate) fn control_intrinsics(&self) -> Vec<ControlIntrinsic> {
        let intrinsics = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.is_multiline())
            .map(|(control_id, runtime)| control_intrinsic_for(control_id, runtime))
            .collect::<Vec<_>>();

        for intrinsic in &intrinsics {
            log_control_intrinsic(intrinsic);
        }

        intrinsics
    }

    pub(crate) fn control_intrinsic(&self, control_id: &str) -> Option<ControlIntrinsic> {
        let runtime = self.runtimes.get(control_id)?;
        runtime
            .is_multiline()
            .then(|| control_intrinsic_for(control_id, runtime))
            .inspect(log_control_intrinsic)
    }
}

fn control_intrinsic_for(control_id: &str, runtime: &TextBoxRuntime) -> ControlIntrinsic {
    ControlIntrinsic {
        control_id: control_id.to_string(),
        current_size: runtime.current_size(),
        min_size: runtime.min_size(),
        desired_size: runtime.desired_size(),
        affects_parent_width: false,
        affects_parent_height: true,
    }
}
