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
