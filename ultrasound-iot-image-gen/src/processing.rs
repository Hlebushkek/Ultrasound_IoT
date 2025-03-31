use std::f32;
use image::{GrayImage, Luma};
use ndarray::{Array2, Array3};
use num_complex::Complex32;

pub fn convert_to_complex(frame: Array3<f32>) -> Array2<Complex32> {
    let (height, width, channels) = frame.dim();
    assert!(channels == 2, "Expected last dimension to be 2 (real, imag)");
    let mut complex_array = Array2::<Complex32>::zeros((height, width));
    for i in 0..height {
        for j in 0..width {
            let real = frame[[i, j, 0]];
            let imag = frame[[i, j, 1]];
            complex_array[[i, j]] = Complex32::new(real, imag);
        }
    }
    complex_array
}

/// Compute the envelope (magnitude) of complex RF data.
pub fn compute_envelope(complex_data: &Array2<Complex32>) -> Array2<f32> {
    complex_data.mapv(|c| c.norm())
}

/// Apply Time Gain Compensation (TGC) to envelope data.
pub fn apply_tgc(envelope: &Array2<f32>) -> Array2<f32> {
    let (num_samples, num_beams) = envelope.dim();
    let mut tgc = envelope.clone();
    for i in 0..num_samples {
        // Compute a gain factor based on the depth index.
        let gain = (1.0 + i as f32 / num_samples as f32).exp();
        for j in 0..num_beams {
            tgc[[i, j]] *= gain;
        }
    }
    tgc
}

/// Apply log compression to the TGC data.
pub fn log_compression(data: &Array2<f32>) -> Array2<u8> {
    let (num_samples, num_beams) = data.dim();
    let mut compressed = Array2::<u8>::zeros((num_samples, num_beams));
    for ((i, j), &value) in data.indexed_iter() {
        // Use logarithmic compression; add 1.0 to avoid log(0)
        let log_val = (1.0 + value).ln();
        // Normalize to 0-255. Adjust divisor if your max log value is different.
        let normalized = (log_val / 5.0 * 255.0).min(255.0).max(0.0);
        compressed[[i, j]] = normalized as u8;
    }
    compressed
}

/// Create a grayscale image from processed data.
pub fn create_image(data: &Array2<u8>) -> GrayImage {
    // The data is in (num_samples, num_beams) order.
    let (num_samples, num_beams) = data.dim();
    let mut img = GrayImage::new(num_beams as u32, num_samples as u32);
    // Map each element to a pixel; note the pixel coordinate mapping.
    for ((i, j), &value) in data.indexed_iter() {
        // Here, j is the horizontal coordinate (beam) and i is the vertical coordinate (depth).
        img.put_pixel(j as u32, i as u32, Luma([value]));
    }
    img
}
