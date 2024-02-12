use banyanfs::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() -> BanyanFsResult<()> {
    Err(BanyanFsError("no main for wasm builds"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> BanyanFsResult<()> {
    use banyanfs::codec::filesystem::DirectoryPermissions;
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
    let actor_id = signing_key.actor_id();
    let mut drive = Drive::initialize_private(&signing_key);

    if !drive.check_accessibility(&signing_key.verifying_key()) {
        tracing::error!("key doesn't have access to the drive");
        return Err(BanyanFsError("key doesn't have access to the drive"));
    }

    if drive.is_writable(&signing_key) {
        let new_perms = DirectoryPermissions::default();
        if let Err(err) = drive.mkdir(actor_id, &["testing", "paths"], new_perms, true) {
            tracing::error!("failed to create directory: {}", err);
            return Ok(());
        }

        match drive.ls(&["testing"]) {
            Ok(dir_contents) => {
                let names: Vec<String> = dir_contents.into_iter().map(|(name, _)| name).collect();
                tracing::info!("dir_contents: {names:?}");
            }
            Err(err) => {
                tracing::error!("failed to list directory: {err}");
                return Ok(());
            }
        }

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
    }

    //let dir_contents = drive.ls("/root/testing")?;
    //tracing::info!("dir_contents: {dir_contents:?}");

    //let mut fh = tokio::fs::File::open("fixtures/minimal.bfs").await?;
    //drive.encode_with_key(&mut fh, &signing_key).await?;
    //fh.close().await?;

    //let mut fh = tokio::fs::File::open("fixtures/minimal.bfs").await?;
    //let loaded_drive = Drive::load_with_key(&mut fh, &signing_key).await?;
    //fh.close().await?;

    //let dir_contents = loaded_drive.ls("/root/testing/deep/paths")?;
    //tracing::info!("dir_contents: {dir_contents:?}");

    Ok(())
}
