//! Various helper method for reporting on the compiled version of the library both from calling
//! applications as well as the version reported in the user agent of the library HTTP clients.

/// Reports the full version and various useful build settings as a well-formatted and
/// semi-structured string.
pub fn full_version() -> String {
    format!(
        "build-profile={} build-timestamp={} features={} repo-version={}",
        env!("BUILD_PROFILE"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_FEATURES"),
        env!("REPO_VERSION"),
    )
}

/// When size matters, but you want to report the version of the library, the returned string from
/// this function is the one for you. It contains only the absolute core version information from
/// the build.
pub fn minimal_version() -> String {
    format!("repo-version={}", env!("REPO_VERSION"),)
}

/// The user agent that will be use by the built-in HTTP client of the library. Can be useful for
/// users of the libraries to check what they'll see in their web server logs.
pub fn user_agent_byte_str() -> Vec<u8> {
    let user_agent_str = minimal_version();
    user_agent_str.as_bytes().to_vec()
}
