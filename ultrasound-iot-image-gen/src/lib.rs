mod processing;
use std::{fmt::Display, io::Cursor};

use ndarray_npy::read_npy;
use processing::*;

use image::{DynamicImage, GenericImageView, ImageOutputFormat};
use ndarray::{Array3, Array4, Axis};

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Record)]
pub struct UltrasoundImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, uniffi::Error)]
pub enum ImageError {
    InvalidData(String),
    GenerationError(String),
}

impl Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            ImageError::GenerationError(msg) => write!(f, "Image generation error: {}", msg),
        }
    }
}

#[derive(Debug, uniffi::Object)]
#[uniffi::export(Debug)]
pub struct ImageProcessor {
    pub path: String
}

#[uniffi::export]
impl ImageProcessor {
    #[uniffi::constructor]
    pub fn new(path: String) -> Self {
        Self { path }
    }
    
    pub fn process(
        &self,
    ) -> Result<UltrasoundImage, ImageError> {
        let raw_data: Array4<f32> =
            read_npy(&self.path).expect("Failed to read npy file");
        println!("Loaded raw data with shape: {:?}", raw_data.dim());

        // For initial processing, select the frame.
        // This gives a 3D array of shape (128, 382, 2)
        let frame_index = 0;
        let frame: Array3<f32> = raw_data.index_axis(Axis(0), frame_index).to_owned();
        println!("Selected frame shape: {:?}", frame.dim());
        
        // Convert the selected frame to a 2D complex array (shape: 128 x 382).
        let complex_data = convert_to_complex(frame);
        println!("Converted frame to complex array with shape: {:?}", complex_data.dim());

        let envelope_data = compute_envelope(&complex_data);
        let tgc_data = apply_tgc(&envelope_data);
        let compressed_data = log_compression(&tgc_data);
        
        let img = create_image(&compressed_data);
        let dyn_img = DynamicImage::ImageLuma8(img);

        let mut buffer = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Png)
            .map_err(|e| ImageError::GenerationError(e.to_string()))?;

        let (width, height) = dyn_img.dimensions();

        Ok(UltrasoundImage {
            data: buffer,
            width,
            height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main() {
        let raw_data: Array4<f32> =
            read_npy("../external/complex_data.npy").expect("Failed to read complex_data.npy");
        println!("Loaded raw data with shape: {:?}", raw_data.dim());

        // This gives a 3D array of shape (128, 382, 2)
        let frame_index = 0;
        let frame: Array3<f32> = raw_data.index_axis(Axis(0), frame_index).to_owned();
        println!("Selected frame shape: {:?}", frame.dim());
        
        // Convert the selected frame to a 2D complex array (shape: 128 x 382).
        let complex_data = convert_to_complex(frame);
        println!("Converted frame to complex array with shape: {:?}", complex_data.dim());

        // 2. Compute the envelope (magnitude) of the complex data.
        let envelope_data = compute_envelope(&complex_data);

        // 3. Apply Time Gain Compensation.
        let tgc_data = apply_tgc(&envelope_data);

        // 4. Apply log compression.
        let compressed_data = log_compression(&tgc_data);

        // 5. Create and save the image.
        let img = create_image(&compressed_data);
        img.save("ultrasound_image_raw.png")
            .expect("Failed to save image");

        println!("Ultrasound image saved as ultrasound_image_raw.png");
    }
}