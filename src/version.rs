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
    format!("{}/{}", base_pkg_name(), env!("REPO_VERSION"))
}

/// The user agent that will be use by the built-in HTTP client of the library.
pub fn user_agent() -> String {
    format!(
        "{}/{} v={}",
        base_pkg_name(),
        env!("CARGO_PKG_VERSION"),
        env!("REPO_VERSION")
    )
}

/// If a downstream binary is compiling in this package, we'll use it as the base name for thing
/// such as the user agent and embedded agent string. Allows for better tracking of format issues
/// if some piece of software needs updating or is misbehaving somehow.
fn base_pkg_name() -> String {
    let base_name = match option_env!("CARGO_BIN_NAME") {
        Some(bn) => bn,
        None => env!("CARGO_PKG_NAME"),
    };

    base_name.to_string()
}
