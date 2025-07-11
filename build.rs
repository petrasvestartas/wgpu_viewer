use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;

fn main() -> Result<()> {
    // Only rerun build script if specific resource files change
    println!("cargo:rerun-if-changed=res/cube.obj");
    println!("cargo:rerun-if-changed=res/cube.mtl");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("res/");
    paths_to_copy.push("assets/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
