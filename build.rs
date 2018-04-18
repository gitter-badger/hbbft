use std::env;
extern crate protoc_rust;

fn generate_proto_rs() {
    let current_dir = env::current_dir().unwrap();
    let protoc_out_dir = current_dir.join("src").join("proto");
    let protoc_input = current_dir.join("proto").join("message.proto");
    let protoc_output = current_dir.join("src").join("proto")
        .join("message.rs");
    let rerun_on_input_change =
        format!("cargo:rerun-if-changed={}", protoc_input.to_str().unwrap());
    let rerun_on_output_change =
        format!("cargo:rerun-if-changed={}", protoc_output.to_str().unwrap());

    println!("{}", rerun_on_input_change);
    println!("{}", rerun_on_output_change);

    protoc_rust::run(protoc_rust::Args {
        out_dir: protoc_out_dir.to_str().unwrap(),
        input: &[protoc_input.to_str().unwrap()],
        includes: &[],
    }).expect("protoc");
}

fn main() {
    generate_proto_rs();
}
