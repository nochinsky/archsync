//! Restore uses fake package managers, so these tests never install packages.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new(snapshot: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("bin")).unwrap();
        fs::write(dir.path().join("archpkglist.toml"), snapshot).unwrap();
        let fixture = Self { dir };
        fixture.add_command("pacman");
        fixture.add_command("sudo");
        fixture
    }

    fn add_command(&self, name: &str) {
        let path = self.dir.path().join("bin").join(name);
        fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' \"{name} $*\" >> \"$ARCHSYNC_TEST_COMMAND_LOG\"\n"),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn restore(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_archsync"))
            .current_dir(self.dir.path())
            .env("PATH", self.dir.path().join("bin"))
            .env(
                "ARCHSYNC_TEST_COMMAND_LOG",
                self.dir.path().join("commands.log"),
            )
            .arg("restore")
            .output()
            .unwrap()
    }

    fn commands(&self) -> String {
        fs::read_to_string(self.dir.path().join("commands.log")).unwrap()
    }
}

#[test]
fn repo_only_snapshot_restores_without_an_aur_helper() {
    for aur in ["", "\n[aur]\n", "\n[aur]\nignored = false\n"] {
        let fixture = Fixture::new(&format!("[repo]\nhello = true\nskipped = false\n{aur}"));
        let output = fixture.restore();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(
            fixture.commands(),
            "pacman --version\nsudo pacman -S --needed hello\n"
        );
        assert!(String::from_utf8(output.stdout)
            .unwrap()
            .contains("Installed hello"));
    }
}

#[test]
fn aur_snapshot_prefers_yay_when_both_helpers_are_available() {
    let fixture = Fixture::new("[repo]\nhello = true\n[aur]\nexample = true\nignored = false\n");
    fixture.add_command("yay");
    fixture.add_command("paru");
    let output = fixture.restore();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fixture.commands(),
        "pacman --version\nyay --version\nsudo pacman -S --needed hello\nyay -S --needed example\n"
    );
}

#[test]
fn aur_snapshot_falls_back_to_paru() {
    let fixture = Fixture::new("[aur]\nexample = true\n");
    fixture.add_command("paru");
    let output = fixture.restore();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fixture.commands(),
        "pacman --version\nparu --version\nparu -S --needed example\n"
    );
}

#[test]
fn missing_required_aur_helper_fails_before_installing_repo_packages() {
    let fixture = Fixture::new("[repo]\nhello = true\n[aur]\nexample = true\n");
    let output = fixture.restore();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Neither 'yay' nor 'paru' found"));
    assert_eq!(fixture.commands(), "pacman --version\n");
}
