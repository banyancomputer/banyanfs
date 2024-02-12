use banyanfs::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() -> BanyanFsResult<()> {
    Err(BanyanFsError("no main for wasm builds"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> BanyanFsResult<()> {
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Layer};

    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env_lossy();

    let stderr_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();
    tracing::debug!("running banyanfs {}", version());

    let mut rng = banyanfs::utils::crypto_rng();

    let header = FormatHeader {
        ecc_present: false,
        private: false,
        filesystem_id: FilesystemId::generate(&mut rng),
    };

    let mut output_stream = Vec::new();
    header.encode(&mut output_stream, 0).await.unwrap();
    tracing::info!("output_stream: {:02x?}", output_stream);

    let signing_key = SigningKey::generate(&mut rng);
    let _drive = Drive::initialize_private(&signing_key);

    //if !drive.check_accessibility(key) {
    //    tracing::error!("key doesn't have access to the drive");
    //    return Err(BanyanFsError("key doesn't have access to the drive"));
    //}

    //drive.unlock(key)?;

    //if drive.is_writable() {
    //    drive.mkdir("/root/testing/deep/paths")?;

    //    let fh = drive.open("/root/testing/deep/paths/file.txt")?;
    //    fh.write(b"hello world")?;
    //    fh.close()?;

    //    let fh = drive.open("/root/testing/deep/paths/file.txt")?;
    //    fh.seek(std::io::SeekFrom::Start(6))?;
    //    let mut buf = [0u8; 5];
    //    fh.read(&mut buf)?;
    //    assert_eq!(&buf, b"world");

    //    drive.delete("/root/testing/deep/paths/file.txt")?;

    //    let new_key: &[u8] = &[0x68, 0x55];
    //    drive.authorize_key(new_key, Permission::StructureRead | Permission::DataRead)?;

    //    drive.sync()?;
    //}

    //let dir_contents = drive.ls("/root/testing")?;
    //tracing::info!("dir_contents: {dir_contents:?}");

    Ok(())
}
