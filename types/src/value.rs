use crate::texture::GpuTexture;
use image::DynamicImage;
use std::sync::{Arc, OnceLock};

/// 数据类型标识符，开放类型。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataType(pub String);

impl DataType {
    pub fn image() -> Self {
        Self("image".into())
    }
    pub fn float() -> Self {
        Self("float".into())
    }
    pub fn int() -> Self {
        Self("int".into())
    }
    pub fn bool() -> Self {
        Self("bool".into())
    }
    pub fn color() -> Self {
        Self("color".into())
    }
    pub fn string() -> Self {
        Self("string".into())
    }
    pub fn handle() -> Self {
        Self("handle".into())
    }
}

/// 外部资源句柄。用于引用 Python 后端或其他运行时持有的对象。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    pub handle_id: String,
    pub data_type: DataType,
    pub backend: String,
    pub bytes: usize,
}

impl Handle {
    pub fn new(
        handle_id: impl Into<String>,
        data_type: DataType,
        backend: impl Into<String>,
        bytes: usize,
    ) -> Self {
        Self {
            handle_id: handle_id.into(),
            data_type,
            backend: backend.into(),
            bytes,
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 图像数据，懒转换。内部持有 CPU 和 GPU 双缓存。
#[derive(Clone)]
pub struct Image {
    cpu: Arc<OnceLock<Arc<DynamicImage>>>,
    gpu: Arc<OnceLock<Arc<GpuTexture>>>,
}

impl Image {
    /// 从 CPU 图像创建
    pub fn from_cpu(img: DynamicImage) -> Self {
        let cpu = OnceLock::new();
        cpu.set(Arc::new(img)).ok();
        Self {
            cpu: Arc::new(cpu),
            gpu: Arc::new(OnceLock::new()),
        }
    }

    /// 从 GPU 纹理创建
    pub fn from_gpu(tex: GpuTexture) -> Self {
        let gpu = OnceLock::new();
        gpu.set(Arc::new(tex)).ok();
        Self {
            cpu: Arc::new(OnceLock::new()),
            gpu: Arc::new(gpu),
        }
    }

    /// 获取 GPU 纹理。首次调用时从 CPU 上传。
    pub fn as_gpu(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> &Arc<GpuTexture> {
        self.gpu.get_or_init(|| {
            let cpu = self.cpu.get().expect("Image has neither CPU nor GPU data");
            Arc::new(GpuTexture::from_dynamic_image(device, queue, cpu))
        })
    }

    /// 获取 CPU 图像。首次调用时从 GPU 回读。
    pub fn as_cpu(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> &Arc<DynamicImage> {
        self.cpu.get_or_init(|| {
            let gpu = self.gpu.get().expect("Image has neither CPU nor GPU data");
            Arc::new(gpu.to_dynamic_image(device, queue))
        })
    }

    /// Get CPU data if available (no GPU needed).
    pub fn cpu_data(&self) -> Option<&Arc<DynamicImage>> {
        self.cpu.get()
    }

    /// Get GPU data if available (no conversion needed).
    pub fn gpu_data(&self) -> Option<&Arc<GpuTexture>> {
        self.gpu.get()
    }

    pub fn has_cpu(&self) -> bool {
        self.cpu.get().is_some()
    }
    pub fn has_gpu(&self) -> bool {
        self.gpu.get().is_some()
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("has_cpu", &self.has_cpu())
            .field("has_gpu", &self.has_gpu())
            .finish()
    }
}

/// 运行时值，节点参数和执行结果的实际数据。
#[derive(Clone, Debug)]
pub enum Value {
    Image(Image),
    Handle(Handle),
    Float(f32),
    Int(i64),
    Bool(bool),
    Color([f32; 4]),
    String(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_equality() {
        assert_eq!(DataType::image(), DataType::image());
        assert_ne!(DataType::image(), DataType::float());
    }

    #[test]
    fn test_data_type_custom() {
        let custom = DataType("my_plugin_type".into());
        assert_ne!(custom, DataType::image());
    }

    #[test]
    fn test_image_from_cpu() {
        let img = image::DynamicImage::new_rgba8(2, 2);
        let image = Image::from_cpu(img);
        assert!(image.has_cpu());
        assert!(!image.has_gpu());
    }

    #[test]
    fn test_value_clone() {
        let v = Value::Float(3.14);
        let v2 = v.clone();
        match v2 {
            Value::Float(f) => assert_eq!(f, 3.14),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_handle_value_clone() {
        let handle = Handle::new("h1", DataType::handle(), "python", 1024);
        let value = Value::Handle(handle.clone());

        match value.clone() {
            Value::Handle(v) => assert_eq!(v, handle),
            _ => panic!("expected Handle"),
        }
    }
}
