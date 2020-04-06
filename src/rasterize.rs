use image::{GenericImageView, ImageBuffer, Rgba, SubImage};
use std::convert::TryInto;
use std::fmt;
use std::slice::Iter;

pub const PIXELS_PER_INCH: f32 = 72.0;
pub const RESOLUTION: f32 = 144.0;
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

pub struct RasterizeArgs<'a> {
    pub image: &'a image::DynamicImage,
    pub paper_width_pixels: f32,
    pub paper_height_pixels: f32,
    pub pages_width: u32,
    pub pages_height: u32,
    pub square_size: f32,
    pub color_depth: ColorDepth,
    pub max_radius: f32,
}

pub fn rasterize(args: RasterizeArgs) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let image = args.image;
    let pages_width = args.pages_width;
    let pages_height = args.pages_height;
    let paper_width_pixels = args.paper_width_pixels;
    let paper_height_pixels = args.paper_height_pixels;
    let square_size = args.square_size;
    let half_square_size = (square_size / 2.0).floor() as i32;
    let square_size_floor = square_size.floor() as u32;
    let color_depth = args.color_depth;
    let max_radius = args.max_radius;
    let pages_width_pixels = (pages_width as f32 * paper_width_pixels).ceil() as u32;
    let pages_height_pixels = (pages_height as f32 * paper_height_pixels).ceil() as u32;

    let image_scaled_to_fit_on_pages = image.resize(
        pages_width_pixels,
        pages_height_pixels,
        image::imageops::Nearest,
    );

    let (scaled_image_width_pixels, scaled_image_height_pixels) =
        image_scaled_to_fit_on_pages.dimensions();

    stdweb::console!(
        log,
        "dimensions of scaled image:",
        scaled_image_width_pixels,
        scaled_image_height_pixels
    );

    let mut pages_pairs = Vec::with_capacity(
        (pages_width * pages_height)
            .try_into()
            .expect("pages_width * pages_height was not able to fit into a usize!"),
    );

    for page_y in 0..pages_height {
        for page_x in 0..pages_width {
            pages_pairs.push((page_x, page_y));
        }
    }

    let mut output_pages = Vec::with_capacity(pages_pairs.len());

    // calculate pages, left-right top-bottom
    // each page is its own sub image
    let pages = pages_pairs.into_iter().filter_map(|(page_x, page_y)| {
        let current_pixel_x: u32 = (page_x as f32 * paper_width_pixels).floor() as u32;
        let current_pixel_y: u32 = (page_y as f32 * paper_height_pixels).floor() as u32;

        // this is kind of horrific and I'm not sure it does exactly what I want.
        // for example if you configure 2x2 pages, and the scaled image can't fit
        let x_span =
            if current_pixel_x + (paper_width_pixels.floor() as u32) < scaled_image_width_pixels {
                Some(paper_width_pixels.floor() as u32)
            } else {
                scaled_image_width_pixels.checked_sub(current_pixel_x)
            };

        let y_span = if current_pixel_y + (paper_height_pixels.floor() as u32)
            < scaled_image_height_pixels
        {
            Some(paper_height_pixels.floor() as u32)
        } else {
            scaled_image_height_pixels.checked_sub(current_pixel_y)
        };

        if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
            let page = SubImage::new(
                &image_scaled_to_fit_on_pages,
                current_pixel_x,
                current_pixel_y,
                x_span,
                y_span,
            );

            Some(page)
        } else {
            None
        }
    });

    let mut pixels_in_square = Vec::with_capacity(square_size.powi(2).ceil() as usize);

    let rasterized_pages = pages.map(|page| {
        // create a dupe of this page on which we will draw circles
        let (page_width_pixels, page_height_pixels) = page.dimensions();
        let mut target_page =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::new(page_width_pixels, page_height_pixels);

        let squares_width = (page_width_pixels as f32 / square_size).ceil() as u32;
        let squares_height = (page_height_pixels as f32 / square_size).ceil() as u32;

        // divide into squares
        for square_y in 0..squares_height {
            for square_x in 0..squares_width {
                let current_pixel_x: u32 = (square_x as f32 * square_size).floor() as u32;
                let current_pixel_y: u32 = (square_y as f32 * square_size).floor() as u32;

                let x_span = if current_pixel_x + square_size_floor < page_width_pixels {
                    Some(square_size_floor)
                } else {
                    page_width_pixels.checked_sub(current_pixel_x)
                };

                let y_span = if current_pixel_y + (square_size_floor) < page_height_pixels {
                    Some(square_size_floor)
                } else {
                    page_height_pixels.checked_sub(current_pixel_y)
                };

                // if the span is nonzero and within the boundary
                if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
                    // for a given square, sample the square form the source page
                    // getting radius and color
                    let square =
                        SubImage::new(&page, current_pixel_x, current_pixel_y, x_span, y_span);

                    pixels_in_square.clear();
                    pixels_in_square.extend(square.pixels().map(|(_, _, pixel)| pixel));

                    let average_pixel_color = match color_depth {
                        ColorDepth::RGB => average_color(&pixels_in_square),
                        ColorDepth::Grayscale => BLACK,
                    };

                    let average_brightness = average_brightness(&pixels_in_square);

                    let radius = radius(average_brightness, max_radius);

                    // write the sampling as a circle to the target page
                    let circle_center = (
                        current_pixel_x as i32 + half_square_size,
                        current_pixel_y as i32 + half_square_size,
                    );

                    imageproc::drawing::draw_filled_circle_mut(
                        &mut target_page,
                        circle_center,
                        radius as i32,
                        average_pixel_color,
                    );
                }
            }
        }

        target_page
    });

    output_pages.extend(rasterized_pages);
    output_pages
}

fn average_color(pixels: &[Rgba<u8>]) -> Rgba<u8> {
    let mut r: usize = pixels[0][0] as usize;
    let mut g: usize = pixels[0][1] as usize;
    let mut b: usize = pixels[0][2] as usize;
    let mut a: usize = pixels[0][3] as usize;

    let pixels_len = pixels.len();

    for pixel in pixels {
        r += pixel[0] as usize;
        g += pixel[1] as usize;
        b += pixel[2] as usize;
        a += pixel[3] as usize;
    }

    Rgba([
        (r / pixels_len) as u8,
        (g / pixels_len) as u8,
        (b / pixels_len) as u8,
        (a / pixels_len) as u8,
    ])
}

fn average_brightness(pixels: &[Rgba<u8>]) -> f32 {
    let i = pixels.len() as f32;
    let mut brightness_nominal = brightness(pixels[0]);

    for pixel in &pixels[1..] {
        brightness_nominal += brightness(*pixel);
    }

    brightness_nominal / i
}

fn radius(average_brightness: f32, max_radius: f32) -> f32 {
    (1.0 - average_brightness) * max_radius
}

pub fn max_radius(raster_size: f32, resolution: f32) -> f32 {
    raster_size * resolution / 2.0
}

pub fn square_size(max_radius: f32) -> f32 {
    2.0 * (max_radius - 1.0) / std::f32::consts::SQRT_2
}

fn brightness(pixel: Rgba<u8>) -> f32 {
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;

    0.299 * r + 0.587 * g + 0.114 * b
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorDepth {
    RGB,
    Grayscale,
}

impl fmt::Display for ColorDepth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ColorDepth::RGB => "RGB",
            ColorDepth::Grayscale => "Grayscale",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaperSize {
    USLetter,
    A4,
    A3,
}

impl PaperSize {
    pub fn width_inches(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).width_inches
    }

    pub fn height_inches(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).height_inches
    }

    pub fn width_pixels(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).width_pixels
    }

    pub fn height_pixels(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).height_pixels
    }

    pub fn from_string(s: String) -> Option<PaperSize> {
        match s.as_str() {
            "US Letter" => Some(PaperSize::USLetter),
            "A4" => Some(PaperSize::A4),
            "A3" => Some(PaperSize::A3),
            _ => None,
        }
    }

    pub fn sizes() -> Iter<'static, Self> {
        const PAPER_SIZES: [PaperSize; 3] = [PaperSize::USLetter, PaperSize::A4, PaperSize::A3];
        PAPER_SIZES.iter()
    }

    fn dimensions_inches(self, orientation: Orientation) -> Size {
        match self {
            PaperSize::USLetter => {
                if orientation == Orientation::Portrait {
                    Size::new(8.5, 11.0, PIXELS_PER_INCH)
                } else {
                    Size::new(11.0, 8.5, PIXELS_PER_INCH)
                }
            }
            PaperSize::A4 => {
                if orientation == Orientation::Portrait {
                    Size::new(8.3, 11.7, PIXELS_PER_INCH)
                } else {
                    Size::new(11.7, 8.3, PIXELS_PER_INCH)
                }
            }
            PaperSize::A3 => {
                if orientation == Orientation::Portrait {
                    Size::new(11.7, 16.5, PIXELS_PER_INCH)
                } else {
                    Size::new(16.5, 11.7, PIXELS_PER_INCH)
                }
            }
        }
    }
}

impl fmt::Display for PaperSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            PaperSize::USLetter => "US Letter",
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
        };
        write!(f, "{}", s)
    }
}

struct Size {
    width_inches: f32,
    height_inches: f32,
    width_pixels: f32,
    height_pixels: f32,
}

impl Size {
    fn new(width_inches: f32, height_inches: f32, pixels_per_inch: f32) -> Self {
        Self {
            width_inches,
            height_inches,
            width_pixels: width_inches * pixels_per_inch,
            height_pixels: height_inches * pixels_per_inch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
        };
        write!(f, "{}", s)
    }
}
