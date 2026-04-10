pub mod context;
pub mod cpu;
pub mod gpu;

pub use context::ExecContext;
pub use cpu::CpuExecutor;
pub use gpu::GpuExecutor;

pub struct ImageExecutor {
    gpu: Option<GpuExecutor>,
    cpu: CpuExecutor,
}

impl ImageExecutor {
    pub fn new(gpu: Option<GpuExecutor>) -> Self {
        Self {
            gpu,
            cpu: CpuExecutor::new(),
        }
    }

    pub fn context(&self) -> ExecContext<'_> {
        ExecContext::new(self.gpu.as_ref(), &self.cpu)
    }

    /// 返回 GPU 执行器的引用（如果存在）
    pub fn gpu_executor(&self) -> Option<&GpuExecutor> {
        self.gpu.as_ref()
    }
}
