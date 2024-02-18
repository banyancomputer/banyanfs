#![allow(clippy::vec_init_then_push)]

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn report_build_profile() {
    println!(
        "cargo:rustc-env=BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
}

fn report_enabled_features() {
    let mut enabled_features: Vec<&str> = Vec::new();

    #[cfg(feature = "default")]
    enabled_features.push("default");

    #[cfg(feature = "banyan-api")]
    enabled_features.push("banyan-api");

    #[cfg(feature = "pem")]
    enabled_features.push("pem");

    #[cfg(feature = "strict")]
    enabled_features.push("strict");

    #[cfg(feature = "tomb-compat")]
    enabled_features.push("tomb-compat");

    if enabled_features.is_empty() {
        enabled_features.push("none");
    }

    println!(
        "cargo:rustc-env=BUILD_FEATURES={}",
        enabled_features.join(",")
    );
}

fn report_repository_version() {
    let git_describe = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty", "--long", "--tags"])
        .output()
        .unwrap();

    let long_version = String::from_utf8(git_describe.stdout).unwrap();
    println!("cargo:rustc-env=REPO_VERSION={}", long_version);

    let build_timestamp = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_timestamp}");
}

fn main() {
    report_repository_version();
    report_build_profile();
    report_enabled_features();
}
