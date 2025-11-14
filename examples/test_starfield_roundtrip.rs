
extern crate exr;

fn main() {
    use exr::prelude::*;
    use std::io::Cursor;

    let file = "tests/images/valid/openexr/LuminanceChroma/StarField.exr";
    println!("Reading: {}", file);

    let read_image = read()
        .no_deep_data().all_resolution_levels().all_channels().all_layers().all_attributes()
        .non_parallel();

    let image = read_image.clone().from_file(file).unwrap();
    println!("Successfully read image with {} layer(s)", image.layer_data.len());

    let mut tmp_bytes = Vec::new();
    println!("Writing image...");
    image.write().non_parallel().to_buffered(Cursor::new(&mut tmp_bytes)).unwrap();
    println!("Successfully wrote {} bytes", tmp_bytes.len());

    println!("Re-reading image...");
    match read_image.from_buffered(Cursor::new(tmp_bytes)) {
        Ok(image2) => println!("SUCCESS! Re-read image"),
        Err(e) => println!("ERROR re-reading: {:?}", e),
    }
}
