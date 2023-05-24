use anyhow::{anyhow, Result};
use image::EncodableLayout;
use indicatif::ProgressBar;
use printpdf::{ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px};
use requestty::{Answer, Question};
use sane::{GetAllDevices, ImageBuffer, ReadToImage};
use sane_scan::Sane;
use std::{fs, io::Write, time::Duration};

mod sane;

fn main() -> Result<()> {
    let sane = Sane::init_1_0()?;

    let loading_devices = ProgressBar::new_spinner();
    loading_devices.enable_steady_tick(Duration::from_millis(120));
    loading_devices.set_message("Loading available devices");
    let devices = sane.get_all_devices()?;
    loading_devices.finish();

    let device_question = Question::select("device")
        .message("Select a device")
        .choices(
            devices
                .iter()
                .map(|d| d.name.to_str())
                .collect::<Result<Vec<&str>, _>>()?,
        )
        .build();
    let selected_device_index = match requestty::prompt_one(device_question)? {
        Answer::ListItem(item) => Ok(item.index),
        _ => Err(anyhow!("expected a ListItem from prompt")),
    }?;
    let mut device = devices
        .get(selected_device_index)
        .ok_or(anyhow!("invalid device index"))?
        .open()?;

    let filename_question = Question::input("filename")
        .message("Enter the output filename")
        .build();
    let filename = match requestty::prompt_one(filename_question)? {
        Answer::String(value) => Ok(value),
        _ => Err(anyhow!("expected a String from prompt")),
    }?;

    let mut images = Vec::new();
    loop {
        let scanning = ProgressBar::new_spinner();
        scanning.enable_steady_tick(Duration::from_millis(120));
        scanning.set_message("Scanning");

        device.start_scan()?;
        let image = device.read_to_image()?;
        images.push(image);

        scanning.finish_with_message("Finished scanning");

        let add_page_question = Question::confirm("add_page")
            .message("Add another page?")
            .build();
        let add_page = match requestty::prompt_one(add_page_question)? {
            Answer::Bool(value) => Ok(value),
            _ => Err(anyhow!("expected a Bool from prompt")),
        }?;
        if !add_page {
            break;
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(filename.clone())?;
    let (document, mut page, mut layer) =
        PdfDocument::new("Document", Mm(210.0), Mm(297.0), "Layer");
    for (i, buffer) in images.iter().enumerate() {
        if i > 0 {
            (page, layer) = document.add_page(Mm(210.0), Mm(297.0), "Layer");
        }
        let layer = document.get_page(page).get_layer(layer);

        let image_object = match buffer {
            ImageBuffer::Luma(image) => ImageXObject {
                width: Px(image.width() as usize),
                height: Px(image.height() as usize),
                color_space: ColorSpace::Greyscale,
                bits_per_component: ColorBits::Bit8,
                interpolate: true,
                image_data: image.as_bytes().to_vec(),
                image_filter: None,
                clipping_bbox: None,
            },
            ImageBuffer::Rgb(image) => ImageXObject {
                width: Px(image.width() as usize),
                height: Px(image.height() as usize),
                color_space: ColorSpace::Rgb,
                bits_per_component: ColorBits::Bit8,
                interpolate: true,
                image_data: image.as_bytes().to_vec(),
                image_filter: None,
                clipping_bbox: None,
            },
        };
        let image = Image::from(image_object);
        image.add_to_layer(layer, ImageTransform::default());
    }
    let bytes = document.save_to_bytes()?;
    file.write_all(&bytes)?;

    Ok(())
}
