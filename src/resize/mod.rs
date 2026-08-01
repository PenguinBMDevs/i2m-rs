use crate::config::ResizeAlgorithm;
use crate::error::{Error, Result};
use crate::image::RgbaImage;

mod interp;
mod others;

pub fn resize(
    image: &RgbaImage,
    new_width: u32,
    new_height: u32,
    algorithm: ResizeAlgorithm,
) -> Result<RgbaImage> {
    if new_width == image.width && new_height == image.height {
        return Ok(image.clone());
    }
    if new_width == 0 || new_height == 0 {
        return Err(Error::Resize("target dimensions must be non-zero".into()));
    }

    match algorithm {
        ResizeAlgorithm::AreaResampling => {
            Ok(others::area_resampling(image, new_width, new_height))
        }
        ResizeAlgorithm::Bilinear => Ok(interp::bilinear(image, new_width, new_height)),
        ResizeAlgorithm::NearestNeighbor => {
            Ok(others::nearest_neighbor(image, new_width, new_height))
        }
        ResizeAlgorithm::Bicubic => Ok(interp::bicubic(image, new_width, new_height)),
        ResizeAlgorithm::Lanczos => Ok(interp::lanczos(image, new_width, new_height)),
        ResizeAlgorithm::Gaussian => Ok(interp::gaussian(image, new_width, new_height)),
        ResizeAlgorithm::Mitchell => Ok(interp::mitchell(image, new_width, new_height)),
        ResizeAlgorithm::BoxFilter => Ok(others::box_filter(image, new_width, new_height)),
        ResizeAlgorithm::IntegralImage => Ok(others::integral_image(image, new_width, new_height)),
        ResizeAlgorithm::ModePooling => Ok(others::mode_pooling(image, new_width, new_height)),
        ResizeAlgorithm::Hermite => Ok(interp::hermite(image, new_width, new_height)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::image::RgbaImage;

    #[test]
    fn resize_to_same_size_clones() {
        let image = RgbaImage::new(4, 4, Color::new(255, 0, 0, 255));
        let resized = resize(&image, 4, 4, ResizeAlgorithm::Bilinear).unwrap();
        assert_eq!(resized.width, 4);
        assert_eq!(resized.height, 4);
        assert_eq!(resized.get(2, 2), Color::new(255, 0, 0, 255));
    }

    #[test]
    fn area_resampling_averages_uniform_color() {
        let image = RgbaImage::new(4, 4, Color::new(128, 64, 32, 255));
        let resized = resize(&image, 1, 1, ResizeAlgorithm::AreaResampling).unwrap();
        assert_eq!(resized.get(0, 0), Color::new(128, 64, 32, 255));
    }
}
