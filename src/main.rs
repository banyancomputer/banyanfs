use banyanfs::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() -> BanyanFsResult<()> {
    Err(BanyanFsError::from("no main for wasm builds"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> BanyanFsResult<()> {
    use banyanfs::filesystem::NodeName;

    use tokio_util::compat::TokioAsyncReadCompatExt;
    use tracing::{debug, error, info, Level};
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
    debug!("running banyanfs {}", full_version());

    let mut rng = banyanfs::utils::crypto_rng();

    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    let actor_id = verifying_key.actor_id();

    let _ = signing_key.to_bytes();

    let drive = Drive::initialize_private(&mut rng, signing_key.clone()).unwrap();
    if !drive.has_read_access(actor_id).await {
        error!("key doesn't have access to the drive");
        return Err(BanyanFsError::from("key doesn't have access to the drive"));
    }

    if drive.has_write_access(actor_id).await {
        let mut root = drive.root().await;

        if let Err(err) = root
            .mkdir(&mut rng, &["testing", "paths", "deeply", "@#($%*%)"], true)
            .await
        {
            error!("failed to create directory: {}", err);
            return Ok(());
        }

        let mut testing_dir = match root.cd(&["testing"]).await {
            Ok(dir) => dir,
            Err(err) => {
                error!("failed to switch directory: {}", err);
                return Ok(());
            }
        };

        if let Err(err) = testing_dir
            .mkdir(&mut rng, &["paths", "deeply", "second"], false)
            .await
        {
            error!("failed to create same directory: {}", err);
            return Ok(());
        }

        // duplicate path as before, folders should already exist and not cause any error
        if let Err(err) = testing_dir
            .mkdir(&mut rng, &["paths", "deeply"], false)
            .await
        {
            error!("failed to create same directory: {}", err);
            return Ok(());
        }

        match testing_dir.ls(&[]).await {
            Ok(contents) => {
                let names: Vec<NodeName> = contents.into_iter().map(|(name, _)| name).collect();
                info!(?names, "contents");
            }
            Err(err) => error!("failed to list directory: {}", err),
        }

        // get a fresh handle on the root directory
        let root = drive.root().await;
        match root.ls(&["testing", "paths", "deeply"]).await {
            Ok(contents) => {
                let names: Vec<NodeName> = contents.into_iter().map(|(name, _)| name).collect();
                info!(?names, "contents");
            }
            Err(err) => error!("failed to list directory: {}", err),
        }

        //let fh = drive.open("/root/testing/deep/paths/file.txt")?;
        //fh.write(b"hello world")?;
        //fh.close()?;

        //let fh = drive.open("/root/testing/deep/paths/file.txt")?;
        //fh.seek(std::io::SeekFrom::Start(6))?;
        //let mut buf = [0u8; 5];
        //fh.read(&mut buf)?;
        //assert_eq!(&buf, b"world");

        //drive.rm("/root/testing/deep/paths/file.txt")?;

        //let new_key = SigningKey::generate(&mut rng);
        //let new_pub_key = new_key.verifying_key();
        //drive.authorize_key(new_pub_key, Permission::StructureRead | Permission::DataRead)?;

        //drive.sync()?;
    }

    let mut file_opts = tokio::fs::OpenOptions::new();

    file_opts.write(true);
    file_opts.create(true);
    file_opts.truncate(true);

    let mut fh = match file_opts.open("fixtures/minimal.bfs").await {
        Ok(fh) => fh.compat(),
        Err(err) => {
            tracing::error!("failed to open file to persist drive: {err}");
            return Ok(());
        }
    };

    if let Err(err) = drive
        .encode(&mut rng, ContentOptions::everything(), &mut fh)
        .await
    {
        tracing::error!("failed to encode drive to file: {err}");
        return Ok(());
    }

    tracing::debug!("persisted drive");

    let mut fh = match tokio::fs::File::open("fixtures/minimal.bfs").await {
        Ok(fh) => fh.compat(),
        Err(err) => {
            tracing::error!("failed to re-open file for loading: {err}");
            return Ok(());
        }
    };

    let drive_loader = DriveLoader::new(&signing_key);
    let loaded_drive = match drive_loader.from_reader(&mut fh).await {
        Ok(d) => d,
        Err(err) => {
            tracing::error!("failed to load drive: {err}");
            return Ok(());
        }
    };

    tracing::info!("loaded drive");

    let mut root_dir = loaded_drive.root().await;

    // todo: should add convenient methods on the drive itself for the directory operations
    match root_dir.ls(&["testing", "paths", "deeply"]).await {
        Ok(dir_contents) => {
            let names: Vec<NodeName> = dir_contents.into_iter().map(|(name, _)| name).collect();
            tracing::info!("dir_contents: {names:?}");
        }
        Err(err) => {
            tracing::error!("failed to list directory from loaded drive: {err}");
            return Ok(());
        }
    }

    if let Err(err) = root_dir
        .mv(&mut rng, &["testing", "paths"], &["new base documents"])
        .await
    {
        tracing::error!("failed to rename file in loaded drive: {err}");
        return Ok(());
    }

    Ok(())
}
