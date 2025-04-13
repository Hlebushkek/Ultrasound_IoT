use ndarray::Array3;

#[derive(Debug, uniffi::Record)]
pub struct Array3Shape {
    pub d0: u32,
    pub d1: u32,
    pub d2: u32,
}

impl Array3Shape {
    pub fn new(vals: (u32, u32, u32)) -> Self {
        Self {
            d0: vals.0,
            d1: vals.1,
            d2: vals.2,
        }
    }

    pub fn from_usize(vals: (usize, usize, usize)) -> Self {
        Self {
            d0: vals.0 as u32,
            d1: vals.1 as u32,
            d2: vals.2 as u32,
        }
    }

    pub fn stride(&self) -> [usize; 3] {
        [self.d0 as usize, self.d1 as usize, self.d2 as usize]
    }
}

#[derive(Debug, uniffi::Record)]
pub struct Array3Data {
    pub shape: Array3Shape,
    pub data: Vec<f64>,
}

impl Array3Data {
    pub fn from_array(arr: Array3<f64>) -> Self {
        let shape = arr.dim();
        let data = arr.into_raw_vec();
        Self {
            shape: Array3Shape::from_usize(shape),
            data,
        }
    }

    pub fn into_array(self) -> Array3<f64> {
        Array3::from_shape_vec(self.shape.stride(), self.data)
            .expect("Data length does not match the provided shape")
    }
}
