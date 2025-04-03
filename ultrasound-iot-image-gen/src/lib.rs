pub mod constants;
pub mod iq2img;
pub mod uniffi_helper;

#[cfg(feature = "rf2iq")]
pub mod rf2iq;

#[cfg(feature = "rf2iq")]
use rf2iq::*;

use std::{fmt::Display, io::Cursor, path::Path, time::Instant};
use tracing::info;

use image::{DynamicImage, GenericImageView, ImageOutputFormat};

use ndarray::{Array, Array1, Array2, Array3, ArrayBase, Dim, OwnedRepr, s};
use ndarray_stats::QuantileExt;

use constants::*;
use iq2img::*;
use uniffi_helper::Array3Data;

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

#[derive(Debug, uniffi::Record)]
pub struct IQData {
    pub preproc: Array3Data,
    pub t_interp: Vec<f64>,
    pub xd: Vec<f64>,
}

#[derive(Debug, uniffi::Object)]
#[uniffi::export(Debug)]
pub struct ImageProcessor {
    pub path: String,
}

#[uniffi::export]
impl ImageProcessor {
    #[uniffi::constructor]
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn process_iq(&self, data: IQData) -> Result<UltrasoundImage, ImageError> {
        let before = Instant::now();

        let preproc_data = data.preproc.into_array();
        let t_interp = Array1::from_iter(data.t_interp.into_iter());
        let xd = Array1::from_iter(data.xd.into_iter());

        let zd = &t_interp * SPEED_SOUND / 2.;

        // beamforming
        let data_beamformed = beamform_df(&preproc_data, &t_interp, &xd);
        info!("Beamformed Data shape = {:?}", data_beamformed.shape());
        let m = data_beamformed.slice(s![0, ..]).sum();
        info!("Beamformed Data sum = {:?}", m);

        // lateral locations of beamformed a-lines
        let xd2 = Array1::<f64>::range(0., N_TRANSMIT_BEAMS as f64, 1.) * ARRAY_PITCH;
        let xd2_max = *xd.max().unwrap();
        let xd2 = xd2 - xd2_max / 2.;

        // envelope detection
        let mut img = Array2::<f64>::zeros(data_beamformed.raw_dim());
        for n in 0..N_TRANSMIT_BEAMS {
            let a_line = data_beamformed.slice(s![n as usize, ..]).into_owned();
            let env = envelope(&a_line);
            let mut img_slice = img.slice_mut(s![n as usize, ..]);
            img_slice.assign(&env);
        }
        info!("Envelope detected Data shape = {:?}", img.shape());

        // log compression
        let dr = 35.0;
        let img_log = log_compress(&img, dr);

        // scan conversion
        let (img_sc, x_sc, z_sc) = scan_convert(&img_log, &xd2, &zd);
        info!("Length of z vector after scan conversion {:?}", z_sc.len());
        info!("Length of x vector after scan conversion {:?}", x_sc.len());
        info!("Scan converted imape shape = {:?}", img_sc.shape());

        let img_sc = transpose(img_sc);

        let img = img_sc.mapv(|x| x as u8);
        let imgx = img.shape()[0] as u32;
        let imgy = img.shape()[1] as u32;
        let imgbuf = image::GrayImage::from_vec(imgy, imgx, img.into_raw_vec());

        // let img_save_path = Path::new("./result.png");
        // imgbuf.clone().unwrap().save(img_save_path).unwrap();

        info!("Elapsed time: {:.2?} s", before.elapsed());
        let dyn_img = DynamicImage::ImageLuma8(imgbuf.unwrap());

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

#[uniffi::export]
#[cfg(feature = "rf2iq")]
impl ImageProcessor {
    pub fn process_rf(&self) -> Result<IQData, ImageError> {
        tracing_subscriber::fmt().init();

        let before = Instant::now();

        // data loading
        let data_path = Path::new(&self.path);
        let data = get_data(&data_path);

        info!("Data shape = {:?}", data.shape());

        let t = Array::range(0., REC_LEN as f64, 1.) / SAMPLE_RATE - TIME_OFFSET;
        let xd = Array::range(0., N_PROBE_CHANNELS as f64, 1.) * ARRAY_PITCH;
        let xd_max = *xd.max().unwrap();
        let xd = xd - xd_max / 2.;

        // preprocessing
        let (preproc_data, t_interp) = preproc(&data, &t, &xd);

        info!("Preprocess Data shape = {:?}", preproc_data.shape());

        info!(
            "Elapsed time before beamforming: {:.2?} s",
            before.elapsed()
        );

        Ok(IQData {
            preproc: Array3Data::from_array(preproc_data),
            t_interp: t_interp.into_raw_vec(),
            xd: xd.into_raw_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "rf2iq")]
    fn processor_test() {
        let proc = ImageProcessor::new("../example_us_bmode_sensor_data.h5".to_owned());
        let iq_data = proc.process_rf().unwrap();
        let img = proc.process_iq(iq_data);

        assert!(img.is_ok());
    }
}
