#![recursion_limit = "1024"]

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, SubImage};
use std::borrow::Borrow;
use std::rc::Rc;
use std::slice::Iter;
use stdweb::js;
use stdweb::web::File;
use yew::services::reader::{FileData, ReaderTask};
use yew::services::ReaderService;
use yew::{
    html, html::ChangeData, Component, ComponentLink, Html, InputData, Properties, ShouldRender,
};

const PIXELS_PER_INCH: f32 = 72.0;

const RESOLUTION: f32 = 144.0;

const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

#[derive(Clone, Copy, Debug, PartialEq)]
enum PaperSize {
    USLetter,
    A4,
    A3,
}

impl PaperSize {
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

    fn width_inches(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).width_inches
    }

    fn height_inches(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).height_inches
    }

    fn width_pixels(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).width_pixels
    }

    fn height_pixels(self, orientation: Orientation) -> f32 {
        self.dimensions_inches(orientation).height_pixels
    }

    fn from_string(s: String) -> Option<PaperSize> {
        match s.as_str() {
            "US Letter" => Some(PaperSize::USLetter),
            "A4" => Some(PaperSize::A4),
            "A3" => Some(PaperSize::A3),
            _ => None,
        }
    }

    fn iterator() -> Iter<'static, Self> {
        const PAPER_SIZES: [PaperSize; 3] = [PaperSize::USLetter, PaperSize::A4, PaperSize::A3];
        PAPER_SIZES.iter()
    }
}

impl std::fmt::Display for PaperSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
enum Orientation {
    Portrait,
    Landscape,
}

impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Backend {
    Image,
    SVG,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Backend::Image => "Image",
            Backend::SVG => "SVG",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColorDepth {
    RGB,
    Grayscale,
}

impl std::fmt::Display for ColorDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            ColorDepth::RGB => "RGB",
            ColorDepth::Grayscale => "Grayscale",
        };
        write!(f, "{}", s)
    }
}

struct ImageBackend {
    link: ComponentLink<Self>,
    props: ImageBackendProps,
    image_urls: Vec<String>,
}

pub enum ImageBackendMsg {
    Rasterize,
}

#[derive(Clone, Properties)]
struct ImageBackendProps {
    pages_width: u32,
    pages_height: u32,
    image: Rc<Option<image::DynamicImage>>,
    raster_size: f32,
    max_radius: f32,
    square_size: f32,
    paper_size: PaperSize,
    orientation: Orientation,
    color_depth: ColorDepth,
}

impl Component for ImageBackend {
    type Message = ImageBackendMsg;
    type Properties = ImageBackendProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        ImageBackend {
            link,
            props,
            image_urls: vec![],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Self::Message::Rasterize => {
                let pages_width_pixels = self.props.pages_width as f32
                    * self.props.paper_size.width_pixels(self.props.orientation);
                let pages_height_pixels = self.props.pages_height as f32
                    * self.props.paper_size.height_pixels(self.props.orientation);

                if let Some(image) = &self.props.image.borrow() {
                    let image_scaled_to_fit_on_pages = image.resize(
                        pages_width_pixels.ceil() as u32,
                        pages_height_pixels.ceil() as u32,
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

                    // calculate pages, left-right top-bottom
                    // each page is its own sub image
                    let mut pages: Vec<SubImage<&DynamicImage>> = vec![];

                    for page_y in 0..self.props.pages_height {
                        for page_x in 0..self.props.pages_width {
                            let current_pixel_x: u32 = (page_x as f32
                                * self.props.paper_size.width_pixels(self.props.orientation))
                            .floor() as u32;
                            let current_pixel_y: u32 = (page_y as f32
                                * self.props.paper_size.height_pixels(self.props.orientation))
                            .floor() as u32;

                            // this is kind of horrific and I'm not sure it does exactly what I want.
                            // for example if you configure 2x2 pages, and the scaled image can't fit
                            let x_span = if current_pixel_x
                                + (self
                                    .props
                                    .paper_size
                                    .width_pixels(self.props.orientation)
                                    .floor() as u32)
                                < scaled_image_width_pixels as u32
                            {
                                Some(
                                    self.props
                                        .paper_size
                                        .width_pixels(self.props.orientation)
                                        .floor() as u32,
                                )
                            } else {
                                (scaled_image_width_pixels as u32).checked_sub(current_pixel_x)
                            };

                            let y_span = if current_pixel_y
                                + (self
                                    .props
                                    .paper_size
                                    .height_pixels(self.props.orientation)
                                    .floor() as u32)
                                < scaled_image_height_pixels as u32
                            {
                                Some(
                                    self.props
                                        .paper_size
                                        .height_pixels(self.props.orientation)
                                        .floor() as u32,
                                )
                            } else {
                                (scaled_image_height_pixels as u32).checked_sub(current_pixel_y)
                            };

                            if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
                                let page = SubImage::new(
                                    &image_scaled_to_fit_on_pages,
                                    current_pixel_x,
                                    current_pixel_y,
                                    x_span,
                                    y_span,
                                );

                                pages.push(page);
                            }
                        }
                    }

                    let mut image_urls = vec![];

                    for page in pages {
                        // create a dupe of this page on which we will draw circles
                        let (page_width_pixels, page_height_pixels) = page.dimensions();
                        let mut target_page = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(
                            page_width_pixels,
                            page_height_pixels,
                        );

                        let square_size = self.props.square_size;

                        let squares_width = (page_width_pixels as f32 / square_size).ceil() as u32;
                        let squares_height =
                            (page_height_pixels as f32 / square_size).ceil() as u32;

                        // divide into squares
                        for square_y in 0..squares_height {
                            for square_x in 0..squares_width {
                                let current_pixel_x: u32 =
                                    (square_x as f32 * square_size).floor() as u32;
                                let current_pixel_y: u32 =
                                    (square_y as f32 * square_size).floor() as u32;

                                let x_span = if current_pixel_x + (square_size.floor() as u32)
                                    < page_width_pixels as u32
                                {
                                    Some(square_size.floor() as u32)
                                } else {
                                    (page_width_pixels as u32).checked_sub(current_pixel_x)
                                };

                                let y_span = if current_pixel_y + (square_size.floor() as u32)
                                    < page_height_pixels as u32
                                {
                                    Some(square_size.floor() as u32)
                                } else {
                                    (page_height_pixels as u32).checked_sub(current_pixel_y)
                                };

                                if let (Some(x_span), Some(y_span)) = (x_span, y_span) {
                                    // for a given square, sample the square form the source page
                                    // getting radius and color
                                    let square = SubImage::new(
                                        &page,
                                        current_pixel_x,
                                        current_pixel_y,
                                        x_span,
                                        y_span,
                                    );

                                    let pixels = square
                                        .pixels()
                                        .map(|(_, _, pixel)| pixel)
                                        .collect::<Vec<Rgba<u8>>>();

                                    let average_pixel = if self.props.color_depth == ColorDepth::RGB
                                    {
                                        average_color(&pixels)
                                    } else {
                                        BLACK
                                    };

                                    let average_brightness = average_brightness(&pixels);

                                    let radius = radius(average_brightness, self.props.max_radius);

                                    // write the sampling as a circle to the target page
                                    let circle_center = (
                                        current_pixel_x as i32 + (square_size / 2.0).floor() as i32,
                                        current_pixel_y as i32 + (square_size / 2.0).floor() as i32,
                                    );

                                    imageproc::drawing::draw_filled_circle_mut(
                                        &mut target_page,
                                        circle_center,
                                        radius as i32,
                                        average_pixel,
                                    );
                                }
                            }
                        }

                        // create a blob_str_url for the target page
                        let dyno = DynamicImage::ImageRgba8(target_page);
                        let target_page_as_subimage: SubImage<&DynamicImage> =
                            SubImage::new(&dyno, 0, 0, page_width_pixels, page_height_pixels);
                        let blob_url_str = image_to_object_url(target_page_as_subimage);
                        image_urls.push(blob_url_str);
                    }

                    self.image_urls = image_urls;
                }

                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div>
                    <button onclick=self.link.callback(|_| Self::Message::Rasterize)>
                { "Rasterize" }
                </button>
            </div>

                <div>
                {
                    for self.image_urls.iter().map(|image_url| {
                        html! {
                            <div style="display: inline;">
                                <a style="display: inline;" href={format!("{}", image_url)} alt={"meh"}>{"download"}</a>
                                <img style="display: inline;" src={format!("{}", image_url)} alt={"meh"}></img>
                                </div>
                        }
                    })
                }
                </div>
            </div>
        }
    }
}

pub struct Model {
    link: ComponentLink<Self>,
    reader: ReaderService,
    tasks: Vec<ReaderTask>,
    pages_width: u32,
    pages_height: u32,
    image: Rc<Option<image::DynamicImage>>,
    raster_size: f32,
    max_radius: f32,
    square_size: f32,
    paper_size: PaperSize,
    orientation: Orientation,
    backend: Backend,
    color_depth: ColorDepth,
}

pub enum Msg {
    FileSelection(Vec<File>),
    FileLoaded(FileData),
    UpdatePageWidth(String),
    UpdatePageHeight(String),
    UpdateRasterSize(String),
    UpdatePaperSize(String),
    UpdateOrientation(String),
    UpdateBackend(String),
    UpdateColorDepth(String),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let raster_size = 0.2;
        let max_radius = max_radius(raster_size, RESOLUTION);
        Model {
            link,
            reader: ReaderService::new(),
            tasks: vec![],
            pages_width: 1,
            pages_height: 1,
            image: Rc::new(None),
            raster_size,
            max_radius,
            square_size: square_size(max_radius),
            paper_size: PaperSize::USLetter,
            orientation: Orientation::Portrait,
            backend: Backend::Image,
            color_depth: ColorDepth::RGB,
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

                self.image = Rc::new(Some(i));

                true
            }

            Msg::UpdatePageWidth(s) => {
                let as_u32 = s.parse::<u32>().unwrap();
                self.pages_width = as_u32;

                stdweb::console!(log, "page width set to", self.pages_width);

                true
            }

            Msg::UpdatePageHeight(s) => {
                let as_u32 = s.parse::<u32>().unwrap();
                self.pages_height = as_u32;

                stdweb::console!(log, "page height set to", self.pages_height);

                true
            }

            Msg::UpdateRasterSize(s) => {
                let as_f32 = s.parse::<f32>().unwrap();
                self.raster_size = as_f32;
                self.max_radius = max_radius(self.raster_size, RESOLUTION);
                self.square_size = square_size(self.max_radius);

                stdweb::console!(log, "raster size set to", self.raster_size);
                stdweb::console!(log, "max_radius set to", self.max_radius);
                stdweb::console!(log, "square_size set to", self.square_size);

                true
            }

            Msg::UpdatePaperSize(s) => {
                stdweb::console!(log, &s);
                if let Some(ps) = PaperSize::from_string(s) {
                    self.paper_size = ps;
                }

                true
            }

            Msg::UpdateOrientation(s) => {
                stdweb::console!(log, &s);
                match s.as_ref() {
                    "Portrait" => self.orientation = Orientation::Portrait,
                    "Landscape" => self.orientation = Orientation::Landscape,
                    _ => unreachable!(),
                }

                true
            }

            Msg::UpdateBackend(s) => {
                match s.as_ref() {
                    "Image" => {
                        stdweb::console!(log, "Image backend selected");
                        self.backend = Backend::Image
                    }
                    "SVG" => {
                        stdweb::console!(log, "SVG backend selected");
                        self.backend = Backend::SVG
                    }
                    _ => unreachable!(),
                }

                true
            }

            Msg::UpdateColorDepth(s) => {
                match s.as_ref() {
                    "RGB" => {
                        stdweb::console!(log, "RGB color selected");
                        self.color_depth = ColorDepth::RGB;
                    }
                    "Grayscale" => {
                        stdweb::console!(log, "Grayscale selected");
                        self.color_depth = ColorDepth::Grayscale;
                    }
                    _ => unreachable!(),
                }

                true
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <a href="https://github.com/ckampfe/rat">{ "source code" }</a>
                <div>
                    {
                        format!("{}in x {}in",
                           self.paper_size.width_inches(self.orientation) * self.pages_width as f32,
                           self.paper_size.height_inches(self.orientation) * self.pages_height as f32
                        )
                    }
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

                <div>
                    <div>
                        { "paper size: " }
                        <select name="paper_size" onchange=self.link.callback(|e: ChangeData| {
                            match e {
                                ChangeData::Select(s) => {
                                    Msg::UpdatePaperSize(s.value().unwrap())
                                },
                                _ => unreachable!()
                            }
                        })>
                        {
                            for PaperSize::iterator().map(|paper_size| {
                                html! {
                                    <option value={ paper_size.to_string() }> { paper_size.to_string() } </option>
                                }
                            })
                        }
                        </select>
                    </div>

                    <div>
                        { "orientation: "}
                        <select name="orientation" onchange=self.link.callback(|e: ChangeData| {
                            match e {
                                ChangeData::Select(s) => {
                                    Msg::UpdateOrientation(s.value().unwrap())
                                },
                                _ => unreachable!()
                            }
                        })>
                           <option value={ Orientation::Portrait.to_string() }> { Orientation::Portrait.to_string() } </option>
                           <option value={ Orientation::Landscape.to_string() }> { Orientation::Landscape.to_string() } </option>
                        </select>
                    </div>

                    <div>
                        { "backend: " }
                        <select name="backend" onchange=self.link.callback(|e: ChangeData| {
                            match e {
                                ChangeData::Select(s) => {
                                    Msg::UpdateBackend(s.value().unwrap())
                                },
                                _ => unreachable!()
                            }
                        })>
                            <option value={ Backend::Image.to_string() }> { Backend::Image.to_string() } </option>
                            // TODO: SVG BACKEND
                            // <option value={ Backend::SVG.to_string() }> { Backend::SVG.to_string() } </option>
                            <option value={ Backend::Image.to_string() }> { "SVG backend coming soon!" } </option>
                        </select>
                    </div>

                    <div>
                        { "color: " }
                        <select name="color_depth" onchange=self.link.callback(|e: ChangeData| {
                            match e {
                                ChangeData::Select(s) => {
                                    Msg::UpdateColorDepth(s.value().unwrap())
                                },
                                _ => unreachable!()
                            }
                        })>
                            <option value={ ColorDepth::RGB.to_string() }> { ColorDepth::RGB.to_string() } </option>
                            <option value={ ColorDepth::Grayscale.to_string() }> { ColorDepth::Grayscale.to_string() } </option>
                        </select>
                    </div>


                    <input type="file" id="input" onchange=self.link.callback(move |v: ChangeData| {
                        let mut res = vec![];

                        if let ChangeData::Files(files) = v {
                            res.extend(files);
                        }

                        Msg::FileSelection(res)
                    }) />

                    <div>{"width (pages)"}</div>
                    <input
                      type="range"
                      name="width"
                      min="1"
                      max="25"
                      value={self.pages_width}
                      oninput=self.link.callback(|e: InputData| Msg::UpdatePageWidth(e.value))/>

                    <div>{"height (pages)"}</div>
                    <input
                      type="range"
                      name="height"
                      min="1"
                      max="25"
                      value={self.pages_height} oninput=self.link.callback(|e: InputData| Msg::UpdatePageHeight(e.value))/>

                    <div>{"raster size"}</div>
                    <input
                      min="0.1"
                      max="5"
                      step="0.05"
                      type="range"
                      name="height"
                      value={self.raster_size}
                      oninput=self.link.callback(|e: InputData| Msg::UpdateRasterSize(e.value))/>

                </div>

                {
                    match self.backend {
                        Backend::Image => {
                            html! {
                                <ImageBackend
                                    image={self.image.clone()}
                                    max_radius={self.max_radius}
                                    orientation={self.orientation}
                                    pages_height={self.pages_height}
                                    pages_width={self.pages_width}
                                    paper_size={self.paper_size}
                                    raster_size={self.raster_size}
                                    square_size={self.square_size}
                                    color_depth={self.color_depth}
                                />
                            }
                        },
                        _ => unreachable!("SVG backend not yet implemented!")
                        // TODO: SVG backend
                        // Backend::SVG => {
                        //     html! {
                        //         <SVGBackend
                        //             image={self.image.clone()}
                        //             max_radius={self.max_radius}
                        //             orientation={self.orientation}
                        //             pages_height={self.pages_height}
                        //             pages_width={self.pages_width}
                        //             paper_size={self.paper_size}
                        //             raster_size={self.raster_size}
                        //             square_size={self.square_size}
                        //         />
                    }
                }
            </div>
        }
    }
}

fn max_radius(raster_size: f32, resolution: f32) -> f32 {
    raster_size * resolution / 2.0
}

fn square_size(max_radius: f32) -> f32 {
    2.0 * (max_radius - 1.0) / std::f32::consts::SQRT_2
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

    for pixel in pixels {
        brightness_nominal += brightness(*pixel);
    }

    brightness_nominal / i
}

fn brightness(pixel: Rgba<u8>) -> f32 {
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;

    0.299 * r + 0.587 * g + 0.114 * b
}

fn radius(average_brightness: f32, max_radius: f32) -> f32 {
    (1.0 - average_brightness) * max_radius
}
