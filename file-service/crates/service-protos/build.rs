use std::{env, fs, path::PathBuf};

fn main() {
    tonic_prost_build::configure();
    // file-service
    let file_protos = vec!["protos/file/file.proto"];
    gen_protos("proto-file-service", file_protos);
}

fn gen_protos(name: &str, inputs: Vec<&str>) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cannot get env OUT_DIR")).join(name);
    fs::create_dir_all(out_dir.clone()).unwrap_or_else(|_| {
        panic!(
            "create dir all {} failed",
            out_dir.to_str().unwrap_or_default()
        )
    });
    tonic_prost_build::configure()
        .out_dir(out_dir)
        .build_server(true)
        .build_client(true)
        .compile_well_known_types(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .build_transport(true)
        .generate_default_stubs(true)
        .compile_protos(inputs.as_slice(), &[""])
        .expect("cannot compile protos");
}
