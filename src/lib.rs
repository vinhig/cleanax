use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use indicatif::ProgressBar;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::wrap_pyfunction;
use rayon::prelude::*;
use std::fs::read_dir;
use std::io;

fn get_color_quantity_with_alpha<T: Sync + num_traits::PrimInt>(
    image: &Vec<T>,
    influence: f32,
    (c1, c2, c3, alpha): (T, T, T, T),
) -> f32 {
    let quantity: f32 = image
        .par_iter()
        .chunks(4)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];
            let a = *pixel[3];

            let r_diff = if r > c1 { r - c1 } else { c1 - r };

            let g_diff = if g > c2 { g - c2 } else { c2 - g };

            let b_diff = if b > c3 { b - c3 } else { c3 - b };

            let a_diff = if a > alpha { a - alpha } else { alpha - b };

            if r_diff.is_zero() && g_diff.is_zero() && b_diff.is_zero() && a_diff.is_zero() {
                influence
            } else {
                0.0
            }
        })
        .sum();

    quantity
}

fn get_color_quantity<T: Sync + num_traits::PrimInt>(
    image: &Vec<T>,
    influence: f32,
    (c1, c2, c3): (T, T, T),
) -> f32 {
    let quantity: f32 = image
        .par_iter()
        .chunks(3)
        .map(|pixel| {
            let r = *pixel[0];
            let g = *pixel[1];
            let b = *pixel[2];

            let r_diff = if r > c1 { r - c1 } else { c1 - r };

            let g_diff = if g > c2 { g - c2 } else { c2 - g };

            let b_diff = if b > c3 { b - c3 } else { c3 - b };

            if r_diff.is_zero() && g_diff.is_zero() && b_diff.is_zero() {
                influence
            } else {
                0.0
            }
        })
        .sum();

    quantity
}

/// Check if given image has more than `quantity` percent of given color.
/// Return true if specified color proportion is greater than `quantity`.
/// A the time being, only RGB and RGBA images are supported. That means
/// an image with another channel format will be marked as deleted.
fn check_solid_color(image: &DynamicImage, color: (u8, u8, u8), quantity: f32) -> bool {
    // Influence of a single pixel
    let influence = 100.0 / (image.width() * image.height()) as f32;
    // Extract pixels
    match image {
        DynamicImage::ImageRgb8(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_color_quantity(buffer, influence, color);

            q > quantity
        }
        DynamicImage::ImageRgba8(pixels) => {
            let buffer = pixels.as_raw();
            let q =
                get_color_quantity_with_alpha(buffer, influence, (color.0, color.1, color.2, 255));

            q > quantity
        }
        DynamicImage::ImageRgb16(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_color_quantity(
                buffer,
                influence,
                (color.0 as u16 * 2, color.1 as u16 * 2, color.2 as u16 * 2),
            );

            q > quantity
        }
        DynamicImage::ImageRgba16(pixels) => {
            let buffer = pixels.as_raw();
            let q = get_color_quantity_with_alpha(
                buffer,
                influence,
                (
                    color.0 as u16 * 2,
                    color.1 as u16 * 2,
                    color.2 as u16 * 2,
                    std::u16::MAX,
                ),
            );

            q > quantity
        }
        DynamicImage::ImageBgr8(pixels) => false,
        DynamicImage::ImageBgra8(pixels) => false,
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
                    
                    // Completely black, red, green, blue and white image are invalid data by definition
                    if check_solid_color(&image, (0, 0, 0), 80.0) {
                        // Delete it -> because black
                        Some(img.to_str().unwrap().to_string())
                    } else if check_solid_color(&image, (255, 255, 255), 80.0) {
                        // Delete it
                        Some(img.to_str().unwrap().to_string())
                    } else if check_solid_color(&image, (255, 0, 0), 80.0) {
                        // Delete it
                        Some(img.to_str().unwrap().to_string())
                    } else if check_solid_color(&image, (0, 255, 0), 80.0) {
                        // Delete it
                        Some(img.to_str().unwrap().to_string())
                    } else if check_solid_color(&image, (0, 0, 255), 80.0) {
                        // Delete it
                        Some(img.to_str().unwrap().to_string())
                    } else {
                        // Keep it
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
