
extern crate exr;

fn main() {
    use exr::prelude::*;

    let file = "tests/images/valid/openexr/LuminanceChroma/Flowers.exr";
    println!("Reading: {}", file);

    match read().no_deep_data().all_resolution_levels().all_channels().all_layers().all_attributes().from_file(file) {
        Ok(image) => {
            println!("SUCCESS! Read image with {} layer(s)", image.layer_data.len());
            for (idx, layer) in image.layer_data.iter().enumerate() {
                println!("Layer {}: size={:?}", idx, layer.size);
            }
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }
}
