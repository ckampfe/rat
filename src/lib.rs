#![recursion_limit = "1024"]

use image::GenericImageView;
use stdweb::js;
use stdweb::web::File;
use yew::services::reader::{FileData, ReaderTask};
use yew::services::ReaderService;
use yew::{html, html::ChangeData, Component, ComponentLink, Html, InputData, ShouldRender};

const PIXELS_PER_INCH: f32 = 72.0;

const PAPER_WIDTH_INCHES: f32 = 8.5;

const PAPER_HEIGHT_INCHES: f32 = 11.0;

const PAGE_PIXELS_WIDE: f32 = PAPER_WIDTH_INCHES * PIXELS_PER_INCH;

const PAGE_PIXELS_TALL: f32 = PAPER_HEIGHT_INCHES * PIXELS_PER_INCH;

const RESOLUTION: f32 = 144.0;

pub struct Model {
    link: ComponentLink<Self>,
    reader: ReaderService,
    tasks: Vec<ReaderTask>,
    files: Vec<FileData>,
    pages_width: u32,
    pages_height: u32,
    image: Option<image::DynamicImage>,
    raster_size: f32,
    max_radius: f32,
    square_size: f32,
    input_pages: Vec<image::SubImage<image::DynamicImage>>,
    image_urls: Vec<String>,
}

pub enum Msg {
    FileSelection(Vec<File>),
    FileLoaded(FileData),
    UpdatePageWidth(String),
    UpdatePageHeight(String),
    UpdateRasterSize(String),
    Rasterize,
}

fn max_radius(raster_size: f32, resolution: f32) -> f32 {
    raster_size * resolution / 2.0
}

/// SquareSize=(float)(2f*((float)MaxRadius-1f)/Math.Sqrt(2f));
fn square_size(max_radius: f32) -> f32 {
    2.0 * (max_radius - 1.0) / std::f32::consts::SQRT_2
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            link,
            reader: ReaderService::new(),
            tasks: vec![],
            files: vec![],
            pages_width: 1,
            pages_height: 1,
            image: None,
            raster_size: 10.0,
            max_radius: max_radius(10.0, RESOLUTION),
            square_size: square_size(max_radius(10.0, RESOLUTION)),
            input_pages: vec![],
            image_urls: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FileSelection(files) => {
                for file in files {
                    let callback = self.link.callback(Msg::FileLoaded);
                    let task = self.reader.read_file(file, callback).unwrap();
                    self.tasks.push(task);
                }

                true
            }

            Msg::FileLoaded(file) => {
                stdweb::console!(log, "finished loading image: {}", &file.name);

                let i = image::load_from_memory(&file.content).unwrap();

                self.image = Some(i);
                self.files.push(file);

                true
            }

            Msg::UpdatePageWidth(s) => {
                let as_u32 = s.parse::<u32>().unwrap();
                self.pages_width = as_u32;
                true
            }

            Msg::UpdatePageHeight(s) => {
                let as_u32 = s.parse::<u32>().unwrap();
                self.pages_height = as_u32;
                true
            }

            Msg::UpdateRasterSize(s) => {
                let as_f32 = s.parse::<f32>().unwrap();
                self.raster_size = as_f32;
                self.max_radius = max_radius(self.raster_size, RESOLUTION);
                self.square_size = square_size(self.max_radius);
                true
            }

            Msg::Rasterize => {
                let pages_width_pixels = self.pages_width as f32 * PAGE_PIXELS_WIDE;
                let pages_height_pixels = self.pages_height as f32 * PAGE_PIXELS_TALL;

                if let Some(image) = &self.image {
                    let image_scaled_to_fit_on_pages = image.resize(
                        pages_width_pixels.ceil() as u32,
                        pages_height_pixels.ceil() as u32,
                        image::imageops::Nearest,
                    );

                    let (sx, sy) = image_scaled_to_fit_on_pages.dimensions();

                    stdweb::console!(log, "scaled dims", sx, sy);

                    // calculate pages, left-right top-bottom
                    // each page is its own sub image
                    let mut pages: Vec<image::SubImage<&image::DynamicImage>> = vec![];

                    stdweb::console!(log, "here0");

                    stdweb::console!(log, "page width", PAGE_PIXELS_WIDE);
                    stdweb::console!(log, "page height", PAGE_PIXELS_TALL);

                    for page_y in 0..self.pages_height {
                        for page_x in 0..self.pages_width {
                            let current_pixel_x: u32 =
                                (page_x as f32 * PAGE_PIXELS_WIDE).floor() as u32;
                            let current_pixel_y: u32 =
                                (page_y as f32 * PAGE_PIXELS_TALL).floor() as u32;

                            stdweb::console!(
                                log,
                                "current pixel x",
                                current_pixel_x,
                                "current pixel y",
                                current_pixel_y
                            );

                            // this is kind of horrific and I'm not sure it does exactly what I want.
                            // for example if you configure 2x2 pages, and the scaled image can't fit
                            let x_span = if current_pixel_x + (PAGE_PIXELS_WIDE.floor() as u32)
                                < sx as u32
                            {
                                Some(PAGE_PIXELS_WIDE.floor() as u32)
                            } else {
                                (sx as u32).checked_sub(current_pixel_x)
                            };

                            let y_span = if current_pixel_y + (PAGE_PIXELS_TALL.floor() as u32)
                                < sy as u32
                            {
                                Some(PAGE_PIXELS_TALL.floor() as u32)
                            } else {
                                (sy as u32).checked_sub(current_pixel_y)
                            };

                            if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
                                stdweb::console!(log, "spans", x_span, y_span);

                                let page = image::SubImage::new(
                                    &image_scaled_to_fit_on_pages,
                                    current_pixel_x,
                                    current_pixel_y,
                                    x_span,
                                    y_span,
                                );

                                let (page_dim_x, page_dim_y) = page.dimensions();

                                stdweb::console!(log, "page dims", page_dim_x, page_dim_y);

                                pages.push(page);
                            }
                        }
                    }

                    let mut image_urls = vec![];

                    for page in pages {
                        // create a dupe of this page on which we will draw circles
                        let (sx, sy) = page.dimensions();
                        let mut target_page =
                            image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(sx, sy);

                        let square_size = self.square_size;

                        let squares_width = (sx as f32 / square_size).ceil() as u32;
                        let squares_height = (sy as f32 / square_size).ceil() as u32;

                        // divide into squares
                        for square_y in 0..squares_height {
                            for square_x in 0..squares_width {
                                let current_pixel_x: u32 =
                                    (square_x as f32 * square_size).floor() as u32;
                                let current_pixel_y: u32 =
                                    (square_y as f32 * square_size).floor() as u32;

                                let x_span = if current_pixel_x + (square_size.floor() as u32) < sx as u32 {
                                    Some(square_size.floor() as u32)
                                } else {
                                    (sx as u32).checked_sub(current_pixel_x)
                                };

                                let y_span = if current_pixel_y + (square_size.floor() as u32) < sy as u32 {
                                    Some(square_size.floor() as u32)
                                } else {
                                    (sy as u32).checked_sub(current_pixel_y)
                                };

                                if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
                                    // for a given square, sample tht square form the source page
                                    // getting radius and color
                                    // float Radius=(1-Brightness/c)*MaxRadius;
                                    let square = image::SubImage::new(
                                        &page,
                                        current_pixel_x,
                                        current_pixel_y,
                                        x_span,
                                        y_span,
                                    );

                                    let pixels = square
                                        .pixels()
                                        .map(|(_, _, pixel)| pixel)
                                        .collect::<Vec<image::Rgba<u8>>>();
                                    let mut r: usize = pixels[0][0] as usize;
                                    let mut g: usize = pixels[0][1] as usize;
                                    let mut b: usize = pixels[0][2] as usize;
                                    let mut a: usize = pixels[0][3] as usize;

                                    let mut i = 0;

                                    for pixel in pixels {
                                        // avg_co;or = imageproc::pixelops::interpolate(avg_color, pixel, 0.5);
                                        i += 1;
                                        r += pixel[0] as usize;
                                        g += pixel[1] as usize;
                                        b += pixel[2] as usize;
                                        a += pixel[3] as usize;
                                    }

                                    let avg_color = image::Rgba([(r / i) as u8, (g / i) as u8, (b / i) as u8, (a / i) as u8]);

                                    // write the sampling as a circle to the target page
                                    let (cx, cy) = (
                                        current_pixel_x as i32 + (square_size / 2.0).floor() as i32,
                                        current_pixel_y as i32 + (square_size / 2.0).floor() as i32,
                                    );

                                    // let left = image::Rgba([100u8, 100u8, 100, 255u8]);
                                    // let right = image::Rgba([0u8, 0u8, 0, 255u8]);
                                    // let avg = imageproc::pixelops::interpolate(left, right, 0.5);

                                    // stdweb::console!(log, avg[0], avg[1], avg[2], avg[3]);

                                    // let solid_blue = image::Rgba([0u8, 0u8, 255u8, 255u8]);
                                    imageproc::drawing::draw_filled_circle_mut(
                                        &mut target_page,
                                        (cx, cy),
                                        10,
                                        avg_color,
                                    );
                                    // pub fn draw_filled_circle_mut<C>(
                                    //     canvas: &mut C,
                                    //     center: (i32, i32),
                                    //     radius: i32,
                                    //     color: C::Pixel
                                    // )
                                    // (5, 0, 0, 0)
                                }
                            }
                        }

                        // create a blob_str_url for the target page

                        // let blob_url_str = image_to_object_url(page);
                        let dyno = image::DynamicImage::ImageRgba8(target_page);
                        let target_page_as_subimage: image::SubImage<&image::DynamicImage> =
                            image::SubImage::new(&dyno, 0, 0, sx, sy);
                        let blob_url_str = image_to_object_url(target_page_as_subimage);
                        image_urls.push(blob_url_str);
                    }

                    self.image_urls = image_urls;
                }

                true
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div>
                    { format!("{}in x {}in", PAPER_WIDTH_INCHES * self.pages_width as f32, PAPER_HEIGHT_INCHES * self.pages_height as f32) }
                </div>

                <div>
                    { format!("{}w x {}h pages", self.pages_width, self.pages_height) }
                </div>

                <div>
                    { format!("max radius: {}", self.max_radius)}
                </div>

                <div>
                    { format!("square size: {}", self.square_size)}
                </div>

                <input type="file" id="input" onchange=self.link.callback(move |v: ChangeData| {
                    let mut res = vec![];

                    if let ChangeData::Files(files) = v {
                        res.extend(files);
                    }

                    Msg::FileSelection(res)
                }) />

                <span>{"width"}</span>
                <input type="range" name="width" min="1" max="25" value={self.pages_width} oninput=self.link.callback(|e: InputData| Msg::UpdatePageWidth(e.value))/>
                <span>{"height"}</span>
                <input type="range" name="height" min="1" max="25" value={self.pages_height} oninput=self.link.callback(|e: InputData| Msg::UpdatePageHeight(e.value))/>

                <span>{"raster size"}</span>
                <input type="text" name="height" value={self.raster_size} oninput=self.link.callback(|e: InputData| Msg::UpdateRasterSize(e.value))/>

                <button onclick=self.link.callback(|_| Msg::Rasterize)>
                   { "Rasterize" }
                </button>

                {
                    for self.image_urls.iter().map(|image_url| {
                        html! {
                            <div>
                                <a href={format!("{}", image_url)} alt={"meh"}>{"download"}</a>
                                <img src={format!("{}", image_url)} alt={"meh"}></img>
                            </div>
                        }
                    })
                }

            </div>
        }
    }
}

fn image_to_object_url(image: image::SubImage<&image::DynamicImage>) -> String {
    let (x, y) = image.dimensions();

    let mut w = std::io::Cursor::new(Vec::new());
    let as_png = image::png::PNGEncoder::new(&mut w);

    let page_as_bytes = image.to_image().into_raw();

    as_png
        .encode(&page_as_bytes, x, y, image::ColorType::Rgba8)
        .unwrap();

    let png_bytes = w.into_inner();

    // https://docs.rs/stdweb/0.4.20/stdweb/struct.UnsafeTypedArray.html
    let png_slice = unsafe { stdweb::UnsafeTypedArray::new(&png_bytes) };
    let blob_url: stdweb::Value = stdweb::js! {
        const slice = @{png_slice};
        const blob = new Blob([slice], { type: "image/png" });
        const imageUrl = URL.createObjectURL(blob);
        return imageUrl
    };

    blob_url.into_string().unwrap()
}
