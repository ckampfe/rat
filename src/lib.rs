#![recursion_limit = "1024"]

const RAT_VERSION: &str = env!("RAT_VERSION");

mod rasterize;

use image::{ImageBuffer, Rgba};
use rasterize::{ColorDepth, Orientation, PaperSize, RESOLUTION};
use std::borrow::Borrow;
use std::fmt;
use std::rc::Rc;
use stdweb::js;
use stdweb::web::File;
use yew::services::reader::{FileData, ReaderTask};
use yew::services::ReaderService;
use yew::{
    html, html::ChangeData, Component, ComponentLink, Html, InputData, Properties, ShouldRender,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Backend {
    Image,
    SVG,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Backend::Image => "Image",
            Backend::SVG => "SVG",
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
                if let Some(image) = self.props.image.borrow() {
                    stdweb::console!(log, "Starting rasterization");
                    let paper_width_pixels =
                        self.props.paper_size.width_pixels(self.props.orientation);
                    let paper_height_pixels =
                        self.props.paper_size.height_pixels(self.props.orientation);

                    let args = rasterize::RasterizeArgs {
                        image,
                        paper_width_pixels,
                        paper_height_pixels,
                        pages_width: self.props.pages_width,
                        pages_height: self.props.pages_height,
                        square_size: self.props.square_size,
                        color_depth: self.props.color_depth,
                        max_radius: self.props.max_radius,
                    };

                    let start = stdweb::js! {
                        return performance.now()
                    };
                    let subimages = rasterize::rasterize(args);

                    let runtime = stdweb::js! {
                        return performance.now() - @{start}
                    };
                    stdweb::console!(log, runtime);

                    let mut image_urls: Vec<String> = vec![];

                    for target_page_as_subimage in subimages {
                        let blob_url_str = image_to_object_url(target_page_as_subimage);
                        image_urls.push(blob_url_str);
                    }

                    self.image_urls = image_urls;

                    true
                } else {
                    stdweb::console!(log, "No image supplied, not rasterizeing anything");
                    false
                }
            }
        }
    }

    // TODO figure out what to do here. Right now we naively rerender on any new props.
    // The reason for this is because `image: Rc<Option<image::DynamicImage>>`
    // does not implement `PartialEq`, otherwise we could derive it for the whole props.
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
        let max_radius = rasterize::max_radius(raster_size, RESOLUTION);
        Model {
            link,
            reader: ReaderService::new(),
            tasks: vec![],
            pages_width: 1,
            pages_height: 1,
            image: Rc::new(None),
            raster_size,
            max_radius,
            square_size: rasterize::square_size(max_radius),
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
                self.max_radius = rasterize::max_radius(self.raster_size, RESOLUTION);
                self.square_size = rasterize::square_size(self.max_radius);

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
                <a href="https://github.com/ckampfe/rat">{ format!("source code version {}", RAT_VERSION) }</a>
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
                            for PaperSize::sizes().map(|paper_size| {
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

fn image_to_object_url(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> String {
    let (x, y) = image.dimensions();

    let mut w = std::io::Cursor::new(Vec::new());
    let as_png = image::png::PNGEncoder::new(&mut w);

    let page_as_bytes = image.into_raw();

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
