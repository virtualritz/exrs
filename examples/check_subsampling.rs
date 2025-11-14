
extern crate exr;

/// Check if test images have channel subsampling
fn main() {
    use exr::prelude::*;

    let files = vec![
        "tests/images/valid/openexr/LuminanceChroma/Garden.exr",
        "tests/images/valid/openexr/LuminanceChroma/Flowers.exr",
    ];

    for file in files {
        println!("\n=== Checking: {} ===", file);

        match MetaData::read_from_file(file, false) {
            Ok(meta) => {
                for (idx, header) in meta.headers.iter().enumerate() {
                    println!("Layer {}: size={}x{}",
                        idx, header.layer_size.width(), header.layer_size.height());

                    for channel in &header.channels.list {
                        println!("  Channel '{}': sampling={:?}, type={:?}",
                            channel.name,
                            channel.sampling,
                            channel.sample_type
                        );

                        if channel.sampling.x() != 1 || channel.sampling.y() != 1 {
                            println!("    >>> SUBSAMPLED CHANNEL DETECTED! <<<");
                        }
                    }
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
    }
}
