fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc");
    // Build scripts run before any threads are spawned.
    unsafe { std::env::set_var("PROTOC", protoc) };
    prost_build::compile_protos(
        &["proto/spacegame2d/protocol/v1/protocol.proto"],
        &["proto"],
    )
    .expect("compile protocol protobuf");
}
