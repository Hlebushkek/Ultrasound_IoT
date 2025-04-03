extern crate basic_dsp;
extern crate blas_src;

use std::path::Path;

use ndarray::{Array, Array1, Array3, s};

use basic_dsp::conv_types::*;
use basic_dsp::*;

use crate::constants::*;

pub fn get_data(data_path: &Path) -> Array3<f64> {
    let file = hdf5::File::open(data_path).unwrap();
    let data = file.dataset("dataset_1").unwrap();
    let data: Array3<f64> = data.read().unwrap();
    return data;
}

pub fn preproc(
    data: &Array3<f64>,
    t: &Array1<f64>,
    xd: &Array1<f64>,
) -> (Array3<f64>, Array1<f64>) {
    // Preprocessing. right now this only does upsampling/interpolation.
    // TODO: filtering, apodization, replace interpolation b/c
    //       it's slow
    let filt_ord = 201;
    let lc = 0.5e6;
    let uc = 2.5e6;
    let lc = lc / (SAMPLE_RATE / 2.0);
    let uc = uc / (SAMPLE_RATE / 2.0);

    let rec_len_interp = REC_LEN * UPSAMP_FACT;
    let mut data_interp = Array3::<f64>::zeros((
        N_TRANSMIT_BEAMS as usize,
        N_PROBE_CHANNELS as usize,
        rec_len_interp as usize,
    ));
    let mut buffer = SingleBuffer::new();
    for n in 0..N_TRANSMIT_BEAMS {
        for m in 0..N_PROBE_CHANNELS {
            // get waveform and convert to DspVec<f64>
            let waveform = data.slice(s![n as usize, m as usize, ..]);
            let mut dsp_vec = waveform.to_owned().into_raw_vec().to_real_time_vec();

            // interpolate - currently a bug(ish) requiring truncation. See https://github.com/liebharc/basic_dsp/issues/46
            dsp_vec
                .interpolatei(&mut buffer, &RaisedCosineFunction::new(0.1), UPSAMP_FACT)
                .unwrap();
            let (mut dsp_vec_data, points) = dsp_vec.get();
            dsp_vec_data.truncate(points);
            // let vec: Vec<f64> = dsp_vec.into(); // This also works but, what if you still need to operate on dsp_vec?

            // plug into new array
            let mut waveform_interp = data_interp.slice_mut(s![n as usize, m as usize, ..]);
            waveform_interp.assign(&Array1::from(dsp_vec_data));
        }
    }
    let sample_rate = SAMPLE_RATE * UPSAMP_FACT as f64;
    let t_interp = Array::range(0.0, rec_len_interp as f64, 1.0) / sample_rate + t[0];

    // remove transmission pulse. truncating before 5 ms would be best,
    let trunc_ind = 350 as usize;
    let data_preproc = data_interp.slice(s![.., .., trunc_ind..]).into_owned();
    let t_interp = t_interp.slice(s![trunc_ind..]).into_owned();

    (data_preproc, t_interp)
}
