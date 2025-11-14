
extern crate exr;

fn main() {
    use exr::prelude::*;
    use std::io::Cursor;

    let file = "tests/images/valid/openexr/IlmfmlmflmTest/comp_b44.exr";
    println!("Reading: {}", file);

    let file_bytes = std::fs::read(file).unwrap();

    let read_image = read()
        .no_deep_data().all_resolution_levels().all_channels().all_layers().all_attributes()
        .non_parallel();

    match read_image.clone().from_buffered(Cursor::new(&file_bytes)) {
        Ok(image) => {
            println!("Successfully read image with {} layer(s)", image.layer_data.len());

            let mut tmp_bytes = Vec::new();
            println!("Writing image...");
            match image.write().non_parallel().to_buffered(Cursor::new(&mut tmp_bytes)) {
                Ok(_) => println!("Successfully wrote {} bytes", tmp_bytes.len()),
                Err(e) => println!("ERROR writing: {:?}", e),
            }
        }
        Err(e) => println!("ERROR reading: {:?}", e),
    }
}
