use image::{io::Reader as ImageReader, DynamicImage};
use indicatif::ProgressBar;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::wrap_pyfunction;
use rayon::prelude::*;
use std::io;
use std::{fs::read_dir, iter::Sum};

#[derive(Debug)]
struct Pixel {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

impl Sum for Pixel {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum: Pixel = Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        for pixel in iter {
            sum.r += pixel.r;
            sum.g += pixel.g;
            sum.b += pixel.b;
            sum.a += pixel.a;
        }

        sum
    }
}

impl Pixel {
    fn div(&self, rhs: u32) -> Self {
        Pixel {
            r: self.r / rhs,
            g: self.g / rhs,
            b: self.b / rhs,
            a: self.a / rhs,
        }
    }

    fn is_zero(&self, threshold: u32) -> bool {
        self.r < threshold && self.g < threshold && self.b < threshold && self.a < threshold
    }
}

fn get_image_statistics_with_alpha<T: Sync + num::traits::PrimInt>(
    image: &Vec<T>,
) -> (Pixel, Pixel) {
    let sum: Pixel = image
        .par_iter()
        .chunks(3)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];
            let a = *pixel[3];

            Pixel {
                r: r.to_u32().unwrap(),
                g: g.to_u32().unwrap(),
                b: b.to_u32().unwrap(),
                a: a.to_u32().unwrap(),
            }
        })
        .sum();

    let mean = sum.div(image.len() as u32 / 4);

    let variance: Pixel = image
        .par_iter()
        .chunks(3)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];
            let a = *pixel[3];

            Pixel {
                r: r.to_u32().unwrap() - mean.r,
                g: g.to_u32().unwrap() - mean.g,
                b: b.to_u32().unwrap() - mean.b,
                a: a.to_u32().unwrap() - mean.a,
            }
        })
        .sum();

    let variance = variance.div(image.len() as u32 / 4);

    (mean, variance)
}

/// Compute mean and standard deviation from colors of given image.
fn get_image_statistics<T: Sync + num::traits::PrimInt>(image: &Vec<T>) -> (Pixel, Pixel) {
    let sum: Pixel = image
        .par_iter()
        .chunks(3)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];
            let a = T::zero();

            Pixel {
                r: r.to_u32().unwrap(),
                g: g.to_u32().unwrap(),
                b: b.to_u32().unwrap(),
                a: a.to_u32().unwrap(),
            }
        })
        .sum();

    let mean = sum.div(image.len() as u32 / 3);

    let variance: Pixel = image
        .par_iter()
        .chunks(3)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];
            let a = T::zero();

            Pixel {
                r: (r.to_u32().unwrap().checked_sub(mean.r).unwrap_or(0)).pow(2),
                g: (g.to_u32().unwrap().checked_sub(mean.g).unwrap_or(0)).pow(2),
                b: (b.to_u32().unwrap().checked_sub(mean.b).unwrap_or(0)).pow(2),
                a: (a.to_u32().unwrap().checked_sub(mean.a).unwrap_or(0)).pow(2),
            }
        })
        .sum();

    let variance = variance.div(image.len() as u32 / 3);

    (mean, variance)
}

/// Check if given image has more than `quantity` percent of given color.
/// Return true if specified color proportion is greater than `quantity`.
/// A the time being, only RGB and RGBA images are supported. That means
/// an image with another channel format will be marked as deleted.
fn check_solid_color(image: &DynamicImage) -> bool {
    // Extract pixels
    match image {
        DynamicImage::ImageRgb8(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_image_statistics(buffer);

            q.1.is_zero(20)
        }
        DynamicImage::ImageRgba8(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_image_statistics_with_alpha(buffer);

            q.1.is_zero(20)
        }
        DynamicImage::ImageRgb16(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_image_statistics(buffer);

            q.1.is_zero(20)
        }
        DynamicImage::ImageRgba16(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_image_statistics_with_alpha(buffer);

            q.1.is_zero(20)
        }
        DynamicImage::ImageBgr8(_) => false,
        DynamicImage::ImageBgra8(_) => false,
        _ => {
            println!("Unsupported image format!");
            true
        }
    }
}

/// Load image from disk.
fn load_image(path: &str) -> Option<DynamicImage> {
    let img = ImageReader::open(path);

    match img {
        Ok(buffer) => match buffer.decode() {
            Ok(dynamic_image) => Some(dynamic_image),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// Select images to delete from root folder.
/// And return deleted images.
#[pyfunction]
fn clean(py: Python, root_folder: String) -> PyResult<&PyList> {
    // List files in root folder
    let images = read_dir(root_folder)
        .expect("Couldn't read dir.")
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    let pb = ProgressBar::new(images.len() as u64);

    let to_delete: Vec<String> = images
        .par_iter()
        .filter_map(|img| {
            let image = load_image(img.to_str().unwrap());

            pb.inc(1);

            match image {
                // If file fails to be loaded as an image
                // Delete it (return its name)
                None => Some(img.to_str().unwrap().to_string()),
                // Otherwise pursue other checks
                Some(image) => {
                    // Take most present color and standard deviation
                    // A very low standard deviation means a solid color image

                    if check_solid_color(&image) {
                        Some(img.to_str().unwrap().to_string())
                    } else {
                        None
                    }
                }
            }
        })
        .collect();

    Ok(PyList::new(py, to_delete))
}

#[pymodule]
fn cleanax(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add_wrapped(wrap_pyfunction!(clean))?;

    Ok(())
}
