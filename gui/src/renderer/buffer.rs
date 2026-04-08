/// 预分配 GPU buffer，按需增长，帧间复用。
///
/// 数据 <= 当前容量时直接 `queue.write_buffer`；
/// 超出时重新创建 2 倍大小的 buffer。
pub struct DynamicBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl DynamicBuffer {
    pub fn new(
        device: &wgpu::Device,
        usage: wgpu::BufferUsages,
        label: &'static str,
        initial_capacity: u64,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: initial_capacity,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity: initial_capacity,
            usage,
            label,
        }
    }

    /// 写入数据。容量不够时重建 buffer（2 倍增长）。
    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        let size = data.len() as u64;
        if size > self.capacity {
            let new_capacity = (size * 2).max(self.capacity * 2);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: new_capacity,
                usage: self.usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = new_capacity;
        }
        queue.write_buffer(&self.buffer, 0, data);
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

// ── 共享 viewport uniform ──────────────────────────────────────────

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ViewportUniform {
    pub size: [f32; 2],
    pub _padding: [f32; 2],
}

/// 主视口 uniform buffer。帧间复用，所有管线共享引用。
pub struct SharedViewport {
    buf: DynamicBuffer,
}

impl SharedViewport {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            buf: DynamicBuffer::new(
                device,
                wgpu::BufferUsages::UNIFORM,
                "shared_viewport_uniform",
                256,
            ),
        }
    }

    /// 每帧调用一次，写入主视口尺寸。
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, viewport_size: [f32; 2]) {
        let uniform = ViewportUniform {
            size: viewport_size,
            _padding: [0.0; 2],
        };
        self.buf.write(device, queue, bytemuck::bytes_of(&uniform));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buf.buffer()
    }
}
