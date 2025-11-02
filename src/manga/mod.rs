use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    io::{BufWriter, Cursor, Read, Write},
};

use flate2::bufread::ZlibDecoder;
use image::{DynamicImage, GrayImage, ImageFormat, RgbImage, RgbaImage};
use lopdf::Document;

pub mod error;

use error::Error;
use error::Result;
use tracing::warn;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub fn extract_images(doc: &Document) -> Result<Vec<(Cow<'_, [u8]>, &'static str)>> {
    let mut images = BTreeMap::new();
    let mut smasks = BTreeSet::new();
    let mut out = Vec::new();

    for (object_id, object) in doc.objects.iter() {
        if let Ok(stream) = object.as_stream()
            && let Ok(subtype) = stream.dict.get(b"Subtype")
            && let Ok(name) = subtype.as_name()
            && name == b"Image"
        {
            let ext = match stream.dict.get(b"Filter").and_then(|f| f.as_name())? {
                b"DCTDecode" => "jpg",
                b"JPXDecode" => "jp2",
                b"FlateDecode" => "png",
                other => {
                    tracing::warn!("Unknown filter: {}", String::from_utf8_lossy(other));
                    "bin"
                }
            };

            if !smasks.iter().any(|v| v == object_id) {
                images.insert(object_id.clone(), ext);
            }

            if let Ok(smask) = stream.dict.get(b"SMask").and_then(|s| s.as_reference()) {
                smasks.insert(smask);
                images.remove(&smask);
            }
        }
    }

    for (image, ext) in images {
        let stream = doc.get_object(image).and_then(|v| v.as_stream())?;

        if matches!(ext, "jpg" | "jp2" | "bin") {
            let img = stream.content.as_slice();
            let img = Cow::Borrowed(img);
            out.push((img, ext));
        } else {
            let width = stream.dict.get(b"Width").and_then(|v| v.as_i64())? as u32;
            let height = stream.dict.get(b"Height").and_then(|v| v.as_i64())? as u32;

            let mut decoder = ZlibDecoder::new(&stream.content[..]);
            let mut content = Vec::new();
            decoder.read_to_end(&mut content)?;
            if let Ok(smask) = stream.dict.get(b"SMask")
                && let Ok(smask) = smask.as_reference()
            {
                let smask = doc
                    .get_object(smask)
                    .and_then(|v| v.as_stream())?
                    .content
                    .as_slice();

                if !stream
                    .dict
                    .get(b"Filter")
                    .and_then(|v| v.as_name())
                    .is_ok_and(|v| v == b"FlateDecode")
                {
                    warn!("Unexpexted smask filter: {:?}", stream.dict.get(b"Filter"))
                }

                let mut decoder = ZlibDecoder::new(smask);
                let mut smask = Vec::new();
                decoder.read_to_end(&mut smask)?;
                content = compose_rgba_png(content.as_slice(), smask.as_slice(), width, height)
            };

            let img_buf = if let Ok(color_space) =
                stream.dict.get(b"ColorSpace").and_then(|v| v.as_name())
            {
                match color_space {
                    b"DeviceGray" => DynamicImage::ImageLuma8(
                        GrayImage::from_raw(width, height, content).ok_or(Error::ImageConvert)?,
                    ),
                    b"DeviceRGB" => DynamicImage::ImageRgb8(
                        RgbImage::from_raw(width, height, content).ok_or(Error::ImageConvert)?,
                    ),
                    _ => DynamicImage::ImageRgba8(
                        RgbaImage::from_raw(width, height, content).ok_or(Error::ImageConvert)?,
                    ),
                }
            } else {
                DynamicImage::ImageRgba8(
                    RgbaImage::from_raw(width, height, content).ok_or(Error::ImageConvert)?,
                )
            };

            let mut img = BufWriter::new(Cursor::new(Vec::new()));
            img_buf.write_to(&mut img, ImageFormat::Png)?;
            let img = match img.into_inner() {
                Ok(img) => img.into_inner(),
                Err(err) => {
                    warn!(?err);
                    continue;
                }
            };

            let img = Cow::Owned(img);
            out.push((img, "png"));
        }
    }

    return Ok(out);
}

fn compose_rgba_png(rgb_data: &[u8], alpha_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut res = Vec::with_capacity((width * height * 4) as usize);

    for (i, chunk) in rgb_data.chunks_exact(3).enumerate() {
        res.extend_from_slice(chunk);
        res.push(alpha_data[i]);
    }

    res
}

pub fn write_images_to_archive(
    images: Vec<(Cow<'_, [u8]>, &'static str)>,
    compression_method: CompressionMethod,
) -> zip::result::ZipResult<Vec<u8>> {
    let mut res = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut res);
    let options = SimpleFileOptions::default().compression_method(compression_method);

    for (i, (image, ext)) in images.into_iter().enumerate() {
        let image = image.as_ref();

        zip.start_file(format!("pg-{}.{ext}", i + 1), options)?;
        zip.write_all(image)?;
    }

    zip.finish()?;

    Ok(res.into_inner())
}
