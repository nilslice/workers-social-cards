use worker::*;

use percent_encoding::percent_decode;
use rusttype::{Font, Scale};
use serde::Deserialize;

mod utils;

#[derive(Deserialize, Default)]
pub struct CardDetails {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub huerot: i32,
}

fn default_title() -> String {
    "Hello from the Edge".to_string()
}

#[event(fetch)]
pub async fn main(req: Request, env: Env) -> Result<Response> {
    utils::set_panic_hook();

    Router::new()
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .get("/card/:title/:huerot", |_, ctx| {
            let title = percent_decode(ctx.param("title").unwrap().as_bytes())
                .decode_utf8()
                .unwrap_or(std::borrow::Cow::Borrowed("<utf8 decode error>"));

            let huerot: i32 = ctx
                .param("huerot")
                .unwrap_or(&"".into())
                .parse()
                .unwrap_or_default();

            if let Ok(mut img) = image::load_from_memory(include_bytes!("card-background.png")) {
                img.huerotate(huerot);
                // Load font.

                if let Some(font) =
                    Font::try_from_vec(Vec::from(include_bytes!("Roboto-Regular.ttf") as &[u8]))
                {
                    // Set font options.
                    let height = 100.0;
                    let mut scale_factor = 1.0;
                    let max_word_length = (1200.0 / height * 1.5) as usize;
                    // Set title if specified.
                    if title.len() > 0 {
                        let longest_word_length = title
                            .split(" ")
                            .max_by_key(|word| word.len())
                            .unwrap_or_default()
                            .len();

                        // Font scaling is necessary because long words are not wrapped.
                        if longest_word_length > max_word_length {
                            scale_factor = (max_word_length as f32) / (longest_word_length as f32);
                        }

                        // Wrapping.
                        let mut i = 0;
                        textwrap::fill(&title, std::cmp::max(max_word_length, longest_word_length))
                            .split("\n")
                            .for_each(|line| {
                                // Create image composite.
                                imageproc::drawing::draw_text_mut(
                                    &mut img,
                                    image::Rgba([255u8, 255u8, 255u8, 255u8]),
                                    60u32,
                                    (40 + i * (height as u32) + 20) as u32,
                                    Scale {
                                        x: height * scale_factor,
                                        y: height * scale_factor,
                                    },
                                    &font,
                                    &line,
                                );
                                i = i + 1;
                            });
                    }
                }

                // Encode image as PNG and write bytes.
                let mut bytes: Vec<u8> = Vec::new();
                img.write_to(&mut bytes, image::ImageOutputFormat::Png)
                    .map_err(|e| Error::RustError(e.to_string()))
                    .unwrap_or_else(|e| console_log!("failed to write image output: {}", e));

                // Respond with image and long cache directives.
                return Response::from_bytes(bytes).map(|resp| {
                    let mut headers = Headers::new();
                    headers
                        .set("content-type", "image/png")
                        .unwrap_or_else(|e| {
                            console_log!("failed to set content-type header: {}", e);
                        });
                    headers
                        .set("cache-control", "public, max-age=31536000, immutable")
                        .unwrap_or_else(|e| {
                            console_log!("failed to set cache-control header: {}", e);
                        });
                    return resp.with_headers(headers);
                });
            } else {
                console_log!("failed to load image data.");
                return Response::error("failed to load social card background image", 500);
            }
        })
        .run(req, env)
        .await
}
