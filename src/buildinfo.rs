pub const OS_NAME: &str = "Tobacco";
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RUST_EDITION: &str = "2021";
pub const BOOT_PROTOCOL: &str = "GRUB Multiboot2 ISO";
pub const KERNEL_MODE: &str = "no_std / no_main";

pub const GIT_COMMIT: &str = match option_env!("TOBACCO_GIT_COMMIT") {
    Some(value) => value,
    None => "local",
};

pub const BUILD_TIME: &str = match option_env!("TOBACCO_BUILD_TIME") {
    Some(value) => value,
    None => "unknown",
};

pub const BUILD_PROFILE: &str = match option_env!("TOBACCO_BUILD_PROFILE") {
    Some(value) => value,
    None => {
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    }
};

pub const BUILD_TARGET: &str = match option_env!("TOBACCO_BUILD_TARGET") {
    Some(value) => value,
    None => "x86_64-unknown-none.json",
};

pub const FEATURE_FLAGS: &str = match option_env!("TOBACCO_BUILD_FEATURES") {
    Some(value) => value,
    None => "none",
};

pub const TOOLCHAIN: &str = match option_env!("TOBACCO_BUILD_TOOLCHAIN") {
    Some(value) => value,
    None => "nightly",
};

pub const REPRODUCIBILITY: &str =
    "build time is derived from git commit timestamp when built in CI";
