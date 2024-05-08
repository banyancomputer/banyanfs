#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    use tokio_util::compat::TokioAsyncReadCompatExt;
    use tracing::{level_filters::LevelFilter, Level};
    use tracing_subscriber::fmt::format::FmtSpan;
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_level(true)
        .with_span_events(FmtSpan::ACTIVE)
        .with_max_level(LevelFilter::from_level(Level::TRACE))
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    use banyanfs::prelude::*;

    println!("running banyanfs {}", full_version());

    let mut rng = banyanfs::utils::crypto_rng();

    let signing_key = std::sync::Arc::new(SigningKey::generate(&mut rng));
    let verifying_key = signing_key.verifying_key();
    let actor_id = verifying_key.actor_id();

    let mut memory_store = MemoryDataStore::default();

    let drive = Drive::initialize_private(&mut rng, signing_key.clone()).unwrap();
    assert!(
        drive.has_read_access(&actor_id).await,
        "creation key to have read access"
    );

    if drive.has_write_access(&actor_id).await {
        let mut root = drive.root().await.unwrap();

        root.mkdir(&mut rng, &["testing", "paths", "deeply", "@#($%*%)"], true)
            .await
            .unwrap();

        let mut testing_dir = root.cd(&["testing"]).await.unwrap();

        testing_dir
            .mkdir(&mut rng, &["paths", "deeply", "second"], false)
            .await
            .unwrap();

        // duplicate path as before, folders should already exist and not cause any error
        testing_dir
            .mkdir(&mut rng, &["paths", "deeply"], false)
            .await
            .unwrap();

        let _contents = testing_dir.ls(&[]).await.unwrap();

        // get a fresh handle on the root directory
        let mut root = drive.root().await.unwrap();
        let _contents = root.ls(&["testing", "paths", "deeply"]).await.unwrap();

        root.write(
            &mut rng,
            &mut memory_store,
            &["testing", "poem-♥.txt"],
            b"a filesystem was born",
        )
        .await
        .unwrap();

        let file_data = root
            .read(&memory_store, &["testing", "poem-♥.txt"])
            .await
            .unwrap();

        assert_eq!(file_data, b"a filesystem was born");

        let additional_key = std::sync::Arc::new(SigningKey::generate(&mut rng));
        let additional_pubkey = additional_key.verifying_key();

        let access = AccessMaskBuilder::maintenance()
            .protected()
            .build()
            .unwrap();

        drive
            .authorize_key(&mut rng, additional_pubkey, access)
            .await
            .unwrap();
    }

    let mut file_opts = tokio::fs::OpenOptions::new();

    file_opts.write(true);
    file_opts.create(true);
    file_opts.truncate(true);

    let mut fh = file_opts
        .open("fixtures/minimal.bfs")
        .await
        .unwrap()
        .compat();

    drive
        .encode(&mut rng, ContentOptions::everything(), &mut fh)
        .await
        .unwrap();

    let mut fh = tokio::fs::File::open("fixtures/minimal.bfs")
        .await
        .unwrap()
        .compat();

    let drive_loader = DriveLoader::new(&signing_key);

    let loaded_drive = drive_loader.from_reader(&mut fh).await.unwrap();
    let mut root_dir = loaded_drive.root().await.unwrap();

    let _contents = root_dir.ls(&["testing", "paths", "deeply"]).await.unwrap();

    root_dir
        .mv(&mut rng, &["testing", "paths"], &["new base documents"])
        .await
        .unwrap();

    let _contents = root_dir.ls(&[]).await.unwrap();

    root_dir
        .rm(&mut memory_store, &["new base documents", "deeply"])
        .await
        .unwrap();
}
