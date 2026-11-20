use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use requestor_core::identity::random_uuid;
use requestor_core::update::UpdatePackage;

use super::{AppBundle, PACKAGE_NAME};

const EXECUTABLE: &str = "requestor-app";

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bundle(folder: &Path, marker: &str) -> PathBuf {
    let bundle = folder.join("Requestor.app");
    let executables = bundle.join("Contents").join("MacOS");
    fs::create_dir_all(&executables).unwrap();
    fs::write(executables.join(EXECUTABLE), marker).unwrap();
    bundle
}

fn package_of(bundle: &Path, folder: &Path) -> UpdatePackage {
    let archive = folder.join(PACKAGE_NAME);
    let packed = Command::new("/usr/bin/ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(bundle)
        .arg(&archive)
        .status()
        .unwrap();
    assert!(packed.success());
    UpdatePackage {
        name: PACKAGE_NAME.to_string(),
        bytes: fs::read(archive).unwrap(),
    }
}

fn marker(bundle: &Path) -> String {
    fs::read_to_string(bundle.join("Contents").join("MacOS").join(EXECUTABLE)).unwrap()
}

fn leftovers(folder: &Path) -> Vec<String> {
    fs::read_dir(folder)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.starts_with('.'))
        .collect()
}

#[test]
fn a_verified_package_replaces_the_bundle_in_place() {
    let scratch =
        Scratch(std::env::temp_dir().join(format!("requestor-updater-{}", random_uuid())));
    let installed = scratch.0.join("Applications");
    let built = scratch.0.join("build");
    let current = bundle(&installed, "old");
    let package = package_of(&bundle(&built, "new"), &scratch.0);
    AppBundle::at(current.clone(), EXECUTABLE)
        .replace_with(&package)
        .unwrap();
    assert_eq!(marker(&current), "new");
    assert!(
        leftovers(&installed).is_empty(),
        "{:?}",
        leftovers(&installed)
    );
}

#[test]
fn a_package_without_the_app_leaves_the_bundle_alone() {
    let scratch =
        Scratch(std::env::temp_dir().join(format!("requestor-updater-{}", random_uuid())));
    let installed = scratch.0.join("Applications");
    let current = bundle(&installed, "old");
    let package = UpdatePackage {
        name: PACKAGE_NAME.to_string(),
        bytes: b"not a zip".to_vec(),
    };
    assert!(AppBundle::at(current.clone(), EXECUTABLE)
        .replace_with(&package)
        .is_err());
    assert_eq!(marker(&current), "old");
    assert!(leftovers(&installed).is_empty());
}
