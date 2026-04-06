use super::cpu::CpuExecutor;
use super::gpu::GpuExecutor;

#[derive(Debug)]
pub struct NoGpuError;

impl std::fmt::Display for NoGpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU not available")
    }
}
impl std::error::Error for NoGpuError {}

#[derive(Clone, Copy)]
pub struct ExecContext<'a> {
    gpu: Option<&'a GpuExecutor>,
    cpu: &'a CpuExecutor,
}

impl<'a> ExecContext<'a> {
    pub fn new(gpu: Option<&'a GpuExecutor>, cpu: &'a CpuExecutor) -> Self {
        Self { gpu, cpu }
    }

    pub fn gpu<T>(&self, f: impl FnOnce(&GpuExecutor) -> T) -> Result<T, NoGpuError> {
        match self.gpu {
            Some(gpu) => Ok(f(gpu)),
            None => Err(NoGpuError),
        }
    }

    pub fn cpu<T>(&self, f: impl FnOnce(&CpuExecutor) -> T) -> T {
        f(self.cpu)
    }

    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_gpu_returns_error() {
        let cpu = CpuExecutor::new();
        let ctx = ExecContext::new(None, &cpu);
        let result = ctx.gpu(|_| 42);
        assert!(result.is_err());
    }

    #[test]
    fn test_cpu_always_available() {
        let cpu = CpuExecutor::new();
        let ctx = ExecContext::new(None, &cpu);
        let val = ctx.cpu(|_| 42);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_has_gpu() {
        let cpu = CpuExecutor::new();
        let ctx = ExecContext::new(None, &cpu);
        assert!(!ctx.has_gpu());
    }
}
