use std::env;
use std::path::PathBuf;

extern crate bindgen;

fn main() {
    println!("cargo:rerun-if-changed=src/bindgen.h");

    let bindings = bindgen::Builder::default()
        .header("src/bindgen.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write generated bindings");
}
