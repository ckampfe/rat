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

pub struct Model {
    link: ComponentLink<Self>,
    reader: ReaderService,
    tasks: Vec<ReaderTask>,
    files: Vec<FileData>,
    pages_width: u32,
    pages_height: u32,
    image: Option<image::DynamicImage>,
    scaled_image: Option<image::DynamicImage>,
    image_blob: Option<stdweb::Value>,
    image_str: Option<String>,
}

pub enum Msg {
    FileSelection(Vec<File>),
    FileLoaded(FileData),
    UpdatePageWidth(String),
    UpdatePageHeight(String),
    Rasterize,
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
            scaled_image: None,
            image_blob: None,
            image_str: None,
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

                let len = file.content.len() as u32;
                stdweb::console!(log, "image bytes len: {}", len);

                let i = image::load_from_memory(&file.content).unwrap();
                let (x, y) = i.dimensions();
                stdweb::console!(log, "image bytes len: {}", x, y);

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
            Msg::Rasterize => {
                let pages_width_pixels =
                    self.pages_width as f32 * PAPER_WIDTH_INCHES * PIXELS_PER_INCH;
                let pages_height_pixels =
                    self.pages_height as f32 * PAPER_HEIGHT_INCHES * PIXELS_PER_INCH;

                if let Some(image) = &self.image {
                    let image_scaled_to_fit_on_pages = image.resize(
                        pages_width_pixels.ceil() as u32,
                        pages_height_pixels.ceil() as u32,
                        image::imageops::Nearest,
                    );

                    let (x, y) = image_scaled_to_fit_on_pages.dimensions();

                    let mut w = std::io::Cursor::new(Vec::new());
                    let as_png = image::png::PNGEncoder::new(&mut w);

                    as_png
                        .encode(
                            &image_scaled_to_fit_on_pages.to_bytes(),
                            x,
                            y,
                            image::ColorType::Rgb8,
                        )
                        .unwrap();

                    stdweb::console!(log, "scaled image bytes len: {}", x, y);

                    let png_bytes = w.into_inner();
                    stdweb::console!(log, "png bytes len", png_bytes.len() as u32);

                    // https://docs.rs/stdweb/0.4.20/stdweb/struct.UnsafeTypedArray.html
                    let png_slice = unsafe { stdweb::UnsafeTypedArray::new(&png_bytes) };
                    let blob_url: stdweb::Value = stdweb::js! {
                        let blob = new Blob([@{png_slice}], { type: "image/png" });
                        let imageUrl = URL.createObjectURL(blob);
                        return imageUrl
                    };

                    let blob_url_str: String = blob_url.into_string().unwrap();

                    stdweb::console!(log, "blob_url_str: ", &blob_url_str);

                    self.image_str = Some(blob_url_str);

                    self.scaled_image = Some(image_scaled_to_fit_on_pages);
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

                <input type="file" id="input" onchange=self.link.callback(move |v: ChangeData| {
                    let mut res = vec![];

                    if let ChangeData::Files(files) = v {
                        res.extend(files);
                    }

                    Msg::FileSelection(res)
                }) />

                <span>{"width"}</span>
                <input type="text" name="width" value={self.pages_width} oninput=self.link.callback(|e: InputData| Msg::UpdatePageWidth(e.value))/>
                <span>{"height"}</span>
                <input type="text" name="height" value={self.pages_height} oninput=self.link.callback(|e: InputData| Msg::UpdatePageHeight(e.value))/>

                <button onclick=self.link.callback(|_| Msg::Rasterize)>
                   { "Rasterize" }
                </button>

                {
                    html! {
                    if let Some(blob) = &self.image_str {
                        html! {
                            <div>
                                <a href={format!("{}", blob)} alt={"meh"}>{"download"}</a>
                                <img src={format!("{}", blob)} alt={"meh"}></img>
                            </div>
                        }
                    } else {
                        html! {
                            <div>{"no"}</div>
                        }
                    }
                    }
                }

            </div>
        }
    }
}
