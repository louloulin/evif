// gRPC proto 编译

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/evif.proto"], &["proto/"])?;

    Ok(())
}
