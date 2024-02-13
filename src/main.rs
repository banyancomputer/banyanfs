use banyanfs::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() -> BanyanFsResult<()> {
    Err(BanyanFsError("no main for wasm builds"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> BanyanFsResult<()> {
    use banyanfs::codec::filesystem::DirectoryPermissions;
    use tokio_util::compat::TokioAsyncReadCompatExt;
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
    let signing_key = SigningKey::generate(&mut rng);

    //let mut drive = Drive::initialize_private(&mut rng, &signing_key);

    //if !drive.check_accessibility(&signing_key.verifying_key()) {
    //    tracing::error!("key doesn't have access to the drive");
    //    return Err(BanyanFsError("key doesn't have access to the drive"));
    //}

    //if drive.is_writable(&signing_key) {
    //    let actor_id = signing_key.actor_id();
    //    let new_perms = DirectoryPermissions::default();

    //    if let Err(err) = drive.mkdir(&mut rng, actor_id, &["testing", "paths"], new_perms, true) {
    //        tracing::error!("failed to create directory: {}", err);
    //        return Ok(());
    //    }

    //    //    let fh = drive.open("/root/testing/deep/paths/file.txt")?;
    //    //    fh.write(b"hello world")?;
    //    //    fh.close()?;

    //    //    let fh = drive.open("/root/testing/deep/paths/file.txt")?;
    //    //    fh.seek(std::io::SeekFrom::Start(6))?;
    //    //    let mut buf = [0u8; 5];
    //    //    fh.read(&mut buf)?;
    //    //    assert_eq!(&buf, b"world");

    //    //    drive.delete("/root/testing/deep/paths/file.txt")?;

    //    //    let new_key: &[u8] = &[0x68, 0x55];
    //    //    drive.authorize_key(new_key, Permission::StructureRead | Permission::DataRead)?;

    //    //    drive.sync()?;
    //}

    //match drive.ls(&["testing"]) {
    //    Ok(dir_contents) => {
    //        let names: Vec<String> = dir_contents.into_iter().map(|(name, _)| name).collect();
    //        tracing::info!("dir_contents: {names:?}");
    //    }
    //    Err(err) => {
    //        tracing::error!("failed to list directory: {err}");
    //        return Ok(());
    //    }
    //}

    //let mut file_opts = tokio::fs::OpenOptions::new();

    //file_opts.write(true);
    //file_opts.create(true);
    //file_opts.truncate(true);

    //let mut fh = match file_opts.open("fixtures/minimal.bfs").await {
    //    Ok(fh) => fh.compat(),
    //    Err(err) => {
    //        tracing::error!("failed to open file: {err}");
    //        return Ok(());
    //    }
    //};

    //if let Err(err) = drive.encode_private(&mut rng, &mut fh, &signing_key).await {
    //    tracing::error!("failed to encode drive: {err}");
    //    return Ok(());
    //}

    let mut fh = match tokio::fs::File::open("fixtures/minimal.bfs").await {
        Ok(fh) => fh.compat(),
        Err(err) => {
            tracing::error!("failed to open file: {err}");
            return Ok(());
        }
    };

    let drive_loader = DriveLoader::new(&signing_key);
    let loaded_drive = match drive_loader.load_from_reader(&mut fh).await {
        Ok(d) => d,
        Err(err) => {
            tracing::error!("failed to load saved drive: {err}");
            return Ok(());
        }
    };

    //match loaded_drive.ls(&["testing"]) {
    //    Ok(dir_contents) => {
    //        let names: Vec<String> = dir_contents.into_iter().map(|(name, _)| name).collect();
    //        tracing::info!("dir_contents: {names:?}");
    //    }
    //    Err(err) => {
    //        tracing::error!("failed to list directory: {err}");
    //        return Ok(());
    //    }
    //}

    Ok(())
}
