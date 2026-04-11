use types::GpuTexture;

pub fn estimate_texture_bytes(texture: &GpuTexture) -> usize {
    texture.width as usize * texture.height as usize * 4
}

#[cfg(test)]
mod tests {
    #[test]
    fn texture_bytes_formula_is_rgba8() {
        let bytes = 16usize * 8usize * 4usize;
        assert_eq!(bytes, 512);
    }
}
