use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const KERNEL_PACKAGE: &str = "cloudos-kernel";
const TARGET: &str = "x86_64-unknown-none";
const PROFILE: &str = "debug";

fn main() -> ExitCode {
    let workspace = workspace_root();
    let cargo = env::var("CARGO").unwrap_or_else(|_| String::from("cargo"));

    let status = Command::new(cargo)
        .current_dir(&workspace)
        .args(["build", "--package", KERNEL_PACKAGE, "--target", TARGET])
        .status()
        .expect("failed to start cargo build for CloudOS kernel");

    if !status.success() {
        return ExitCode::from(status.code().unwrap_or(1) as u8);
    }

    let kernel = workspace
        .join("target")
        .join(TARGET)
        .join(PROFILE)
        .join(KERNEL_PACKAGE);

    let bootimage = workspace
        .join("target")
        .join(TARGET)
        .join(PROFILE)
        .join(format!("bootimage-{KERNEL_PACKAGE}.bin"));

    fs::create_dir_all(bootimage.parent().expect("bootimage path has no parent"))
        .expect("failed to create bootimage output directory");

    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&bootimage)
        .expect("failed to create CloudOS BIOS boot image");

    println!("Created boot image: {}", bootimage.display());

    ExitCode::SUCCESS
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bootimage builder must live inside the workspace")
        .to_path_buf()
}
