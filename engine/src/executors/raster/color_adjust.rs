use std::collections::HashMap;

use image::{DynamicImage, Rgba};
use types::Value;

use crate::capability::{Capability, CapabilityId, SideEffect};
use crate::execution::{ExecutionOutputs, ExecutorError, NodeExecutionRequest};
use crate::executors::{Executor, ExecutorFuture, LocalityProfile};

pub struct ColorAdjustExecutor;

impl ColorAdjustExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ColorAdjustExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for ColorAdjustExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "raster.color_adjust",
            vec![types::DataType::image()],
            vec![types::DataType::image()],
            vec![SideEffect::None],
        )]
    }

    fn execute<'a>(
        &'a self,
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            if cap_id != "raster.color_adjust" {
                return Err(ExecutorError::Unavailable {
                    message: format!("unsupported capability '{cap_id}'"),
                });
            }

            let input = match req.inputs.get("image") {
                Some(Value::Image(image)) => image,
                _ => {
                    return Err(ExecutorError::InvalidInput {
                        message: "color_adjust requires an 'image' input".into(),
                    });
                }
            };
            let cpu = input
                .cpu_data()
                .ok_or_else(|| ExecutorError::MaterializationFailed {
                    message: "color_adjust currently requires CPU-backed images".into(),
                })?;

            let brightness = float_param(&req.inputs, "brightness", 0.0)?;
            let contrast = float_param(&req.inputs, "contrast", 1.0)?;
            let saturation = float_param(&req.inputs, "saturation", 1.0)?;

            let adjusted = adjust_image(cpu.as_ref(), brightness, contrast, saturation);
            Ok(ExecutionOutputs::full(HashMap::from([(
                String::from("image"),
                Value::Image(types::Image::from_cpu(adjusted)),
            )])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Cpu
    }
}

fn float_param(
    inputs: &HashMap<String, Value>,
    key: &str,
    default: f32,
) -> Result<f32, ExecutorError> {
    match inputs.get(key) {
        Some(Value::Float(value)) => Ok(*value),
        Some(Value::Int(value)) => Ok(*value as f32),
        Some(_) => Err(ExecutorError::InvalidInput {
            message: format!("parameter '{key}' must be numeric"),
        }),
        None => Ok(default),
    }
}

fn adjust_image(
    source: &DynamicImage,
    brightness: f32,
    contrast: f32,
    saturation: f32,
) -> DynamicImage {
    let mut rgba = source.to_rgba8();
    for pixel in rgba.pixels_mut() {
        let adjusted = adjust_pixel(*pixel, brightness, contrast, saturation);
        *pixel = adjusted;
    }
    DynamicImage::ImageRgba8(rgba)
}

fn adjust_pixel(pixel: Rgba<u8>, brightness: f32, contrast: f32, saturation: f32) -> Rgba<u8> {
    let alpha = pixel[3];
    let mut rgb = [
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
    ];

    for channel in &mut rgb {
        *channel = ((*channel - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);
    }

    let luma = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
    for channel in &mut rgb {
        *channel = (luma + (*channel - luma) * saturation).clamp(0.0, 1.0);
    }

    Rgba([
        (rgb[0] * 255.0).round() as u8,
        (rgb[1] * 255.0).round() as u8,
        (rgb[2] * 255.0).round() as u8,
        alpha,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturation_zero_grayscales_image() {
        let pixel = Rgba([255, 0, 0, 255]);
        let adjusted = adjust_pixel(pixel, 0.0, 1.0, 0.0);

        assert_eq!(adjusted[0], adjusted[1]);
        assert_eq!(adjusted[1], adjusted[2]);
        assert_eq!(adjusted[3], 255);
    }
}
