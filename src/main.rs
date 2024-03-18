use banyanfs::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() -> BanyanFsResult<()> {
    Err(BanyanFsError::from("no main for wasm builds"))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> BanyanFsResult<()> {
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

    let signing_key = std::sync::Arc::new(SigningKey::generate(&mut rng));
    let verifying_key = signing_key.verifying_key();
    let actor_id = verifying_key.actor_id();

    let _ = signing_key.to_bytes();

    let mut memory_store = MemoryStore::default();

    let drive = Drive::initialize_private(&mut rng, signing_key.clone()).unwrap();
    if !drive.has_read_access(actor_id).await {
        error!("key doesn't have access to the drive");
        return Err(BanyanFsError::from("key doesn't have access to the drive"));
    }

    if drive.has_write_access(actor_id).await {
        let mut root = drive.root().await.map_err(|_| "root unavailable")?;

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
            Ok(contents) => info!(?contents, "contents"),
            Err(err) => error!("failed to list directory: {}", err),
        }

        // get a fresh handle on the root directory
        let mut root = drive.root().await.map_err(|_| "root unavailable")?;
        match root.ls(&["testing", "paths", "deeply"]).await {
            Ok(contents) => info!(?contents, "contents"),
            Err(err) => error!("failed to list directory: {}", err),
        }

        if let Err(err) = root
            .write(
                &mut rng,
                &mut memory_store,
                &["testing", "poem.txt"],
                b"a filesystem was born",
            )
            .await
        {
            error!("failed to write file to drive: {err:?}");
            return Ok(());
        };

        let file_data = match root.read(&memory_store, &["testing", "poem.txt"]).await {
            Ok(data) => data,
            Err(err) => {
                error!("failed to read file from drive: {err:?}");
                return Ok(());
            }
        };

        assert_eq!(file_data, b"a filesystem was born");
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

    let mut root_dir = loaded_drive.root().await.map_err(|_| "root unavailable")?;

    // todo: should add convenient methods on the drive itself for the directory operations
    match root_dir.ls(&["testing", "paths", "deeply"]).await {
        Ok(dir_contents) => {
            tracing::info!("dir_contents: {dir_contents:?}");
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

    let root_contents = match root_dir.ls(&[]).await {
        Ok(dir_contents) => dir_contents,
        Err(err) => {
            tracing::error!("failed to list directory from loaded drive: {err}");
            return Ok(());
        }
    };

    tracing::info!(?root_contents, "root contents after move");

    if let Err(err) = root_dir
        .rm(&mut rng, &["new base documents", "deeply"])
        .await
    {
        tracing::error!("failed to remove directory from loaded drive: {err}");
        return Ok(());
    }

    Ok(())
}
