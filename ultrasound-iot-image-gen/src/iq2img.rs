use std::path::Path;

use image::{GrayImage, imageops::FilterType};
use ndarray::{Array1, Array2, Array3, Zip, s};
use ndarray_linalg::{Scalar, c64};
use ndarray_stats::QuantileExt;

use rustfft::FftPlanner;
use tracing::info;

use crate::constants::*;

fn fft_priv(x: &Array1<c64>, n: usize, inverse: bool) -> Array1<c64> {
    // Copy input into a mutable buffer; pad with zeros if necessary.
    let mut buffer: Vec<c64> = {
        let mut vec = x.to_vec();
        vec.resize(n, c64::new(0.0, 0.0));
        vec
    };

    // Create the FFT planner.
    let mut planner = FftPlanner::<f64>::new();
    let fft = if inverse {
        planner.plan_fft_inverse(n)
    } else {
        planner.plan_fft_forward(n)
    };

    // Execute the FFT in-place.
    fft.process(&mut buffer);

    // If performing an inverse FFT, normalize the result.
    if inverse {
        let scale = 1.0 / n as f64;
        for v in buffer.iter_mut() {
            *v = *v * scale;
        }
    }
    Array1::from(buffer)
}

pub fn fft(x: &Array1<c64>, n: usize) -> Array1<c64> {
    fft_priv(x, n, false)
}

pub fn ifft(x: &Array1<c64>) -> Array1<c64> {
    fft_priv(x, x.len(), true)
}

pub fn array_indexing_1d(x: &Array1<f64>, ind: &Array1<usize>) -> Array1<f64> {
    Zip::from(ind).map_collect(|idx| x[*idx])
}

pub fn beamform_df(data: &Array3<f64>, time: &Array1<f64>, xd: &Array1<f64>) -> Array2<f64> {
    // acoustic propagation distance from transmission to reception for each
    // element. Note: transmission is consdiered to arise from the center
    // of the array.
    let zd = time * SPEED_SOUND / 2.0;
    let zd2 = zd.mapv(|x| x.powi(2));
    let mut prop_dist = Array2::<f64>::zeros((N_PROBE_CHANNELS as usize, zd.len()));
    for r in 0..N_PROBE_CHANNELS {
        let dist = (xd[r as usize].powi(2) + &zd2).mapv(<f64>::sqrt) + &zd;
        let mut slice = prop_dist.slice_mut(s![r as usize, ..]);
        slice.assign(&dist);
    }

    let sample_rate = SAMPLE_RATE * UPSAMP_FACT as f64;
    let prop_dist_ind = (prop_dist / SPEED_SOUND * sample_rate).mapv(|x| x.round() as usize);

    // replace out-of-bounds indices
    let prop_dist_ind = prop_dist_ind.mapv(|x| x.min(time.len() - 1));

    // beamform
    let mut image = Array2::<f64>::zeros((N_TRANSMIT_BEAMS as usize, zd.len()));
    for n in 0..N_TRANSMIT_BEAMS {
        let mut scan_line = Array1::<f64>::zeros(zd.len());
        for m in 0..N_PROBE_CHANNELS {
            let waveform = data.slice(s![n as usize, m as usize, ..]).into_owned();
            let inds = prop_dist_ind.slice(s![m as usize, ..]).into_owned();
            let waveform_indexed = array_indexing_1d(&waveform, &inds);
            scan_line += &waveform_indexed;
        }
        let mut image_slice = image.slice_mut(s![n as usize, ..]);
        image_slice.assign(&scan_line);
    }
    return image;
}

pub fn log_compress(data: &Array2<f64>, dr: f64) -> Array2<f64> {
    let data_max = *(data.max().unwrap());
    let data_log = 20.0 * data.mapv(|x| (x / data_max).log10());
    let data_log = data_log.mapv(|x| x.max(-dr));
    let data_log = (data_log + dr) / dr;
    data_log
}

pub fn ndarray_to_gray_image(x: &Array2<f64>) -> GrayImage {
    let height = x.shape()[0];
    let width = x.shape()[1];
    // Convert the ndarray elements to u8.
    println!("{:?}", x);
    let data: Vec<u8> = x.iter().map(|&v| (v * 255f64) as u8).collect();
    // println!("{:?}", data);
    // Create a GrayImage from the raw vector.
    GrayImage::from_vec(width as u32, height as u32, data)
        .expect("Failed to create GrayImage from ndarray")
}

fn resize_ndarray(img_src: &Array2<f64>, fx: f64, fy: f64) -> Array2<f64> {
    // Determine the original dimensions.
    let orig_height = img_src.shape()[0] as f64;
    let orig_width = img_src.shape()[1] as f64;
    // Compute the new dimensions.
    let new_height = (orig_height * fy).round() as u32;
    let new_width = (orig_width * fx).round() as u32;

    // Convert the ndarray to a GrayImage.
    let gray_img = ndarray_to_gray_image(img_src);
    // Resize using a linear-like filter (Triangle filter here).
    let resized_img =
        image::imageops::resize(&gray_img, new_width, new_height, FilterType::Triangle);

    // Convert the resized GrayImage back into an Array2<f64>.
    let resized_data: Vec<f64> = resized_img
        .into_vec()
        .into_iter()
        .map(|v| v as f64)
        .collect();
    Array2::from_shape_vec((new_height as usize, new_width as usize), resized_data)
        .expect("Failed to convert resized image back to ndarray")
}

pub fn scan_convert(
    img: &Array2<f64>,
    x: &Array1<f64>,
    z: &Array1<f64>,
) -> (Array2<f64>, Array1<f64>, Array1<f64>) {
    // decimate in depth dimensions
    let img_decim = img.slice(s![.., ..;DECIM_FACT]).into_owned();
    let z_sc = z.slice(s![..;DECIM_FACT]).into_owned();

    info!("Decimated imape shape = {:?}", img_decim.shape());

    // make pixels square by resampling in x-direction
    let dz = z_sc[1] - z_sc[0];
    let dx = x[1] - x[0];
    let img_sc = resize_ndarray(&img_decim, 1., dx / dz);
    let x_sc = Array1::<f64>::linspace(x[0], x[x.len() - 1], img_sc.shape()[0]);

    (img_sc, x_sc, z_sc)
}

pub fn transpose(a: Array2<f64>) -> Array2<f64> {
    // transpose a 2-d array while maintining c-order layout
    let a_t = a.t();
    let mut a_t_owned = Array2::zeros(a_t.raw_dim());
    a_t_owned.assign(&a_t);
    a_t_owned
}

pub fn img_save(img: &Array2<f64>, img_save_path: &Path) {
    let img = img.mapv(|x| x as u8);
    let imgx = img.shape()[0] as u32;
    let imgy = img.shape()[1] as u32;
    let imgbuf = image::GrayImage::from_vec(imgy, imgx, img.into_raw_vec());
    imgbuf.unwrap().save(img_save_path).unwrap();
}

pub fn analytic(waveform: &Array1<f64>, nfft: usize) -> Array1<c64> {
    // Discrete-time analytic signal
    // This mimics scipy.signal.hilbert

    let waveform = waveform.mapv(|x| c64::new(x, 0.0)); // convert to complex
    let waveform_fft = fft(&waveform, nfft);

    // currently only working if nfft is even
    let mut h1 = Array1::<f64>::ones(nfft);
    let h2 = Array1::<f64>::ones(((nfft / 2) - 1) as usize) * 2.0;
    let mut slice = h1.slice_mut(s![1..(nfft / 2)]);
    slice.assign(&h2);
    let h0 = Array1::<f64>::zeros((nfft / 2 - 1) as usize);
    let mut slice = h1.slice_mut(s![(nfft / 2) + 1..]);
    slice.assign(&h0);

    let analytic_fft = waveform_fft * h1.mapv(|x| c64::new(x, 0.0));
    let analytic = ifft(&analytic_fft);

    analytic
}

pub fn envelope(waveform: &Array1<f64>) -> Array1<f64> {
    let nfft = 6340; // length of data
    let env = analytic(&waveform, nfft).mapv(|x| x.abs());
    let env = env.slice(s![..waveform.len()]).to_owned();

    env
}
