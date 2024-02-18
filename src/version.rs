pub fn full_version() -> String {
    format!(
        "build-profile={} build-timestamp={} features={} repo-version={}",
        env!("BUILD_PROFILE"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_FEATURES"),
        env!("REPO_VERSION"),
    )
}

pub fn minimal_version() -> String {
    format!("repo-version={}", env!("REPO_VERSION"),)
}
