
extern crate exr;

fn main() {
    use exr::prelude::*;

    let file = "tests/images/valid/openexr/IlmfmlmflmTest/comp_b44.exr";
    println!("Reading: {}", file);

    match read().no_deep_data().all_resolution_levels().all_channels().all_layers().all_attributes().from_file(file) {
        Ok(image) => {
            println!("Successfully read image with {} layer(s)", image.layer_data.len());
            for (idx, layer) in image.layer_data.iter().enumerate() {
                println!("Layer {}: size={:?}", idx, layer.size);
                for (ch_idx, channel) in layer.channel_data.list.iter().enumerate() {
                    println!("  Channel {}: name={}, sampling={:?}",
                        ch_idx, channel.name, channel.sampling);
                }
            }
        }
        Err(e) => println!("ERROR reading: {:?}", e),
    }
}
