pub fn version() -> String {
    format!(
        "build-profile={} build-timestamp={} features={} repo-version={}",
        env!("BUILD_PROFILE"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_FEATURES"),
        env!("REPO_VERSION"),
    )
}
