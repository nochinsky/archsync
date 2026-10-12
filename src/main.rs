use clap::{Parser, Subcommand};
use colored::*;
use std::{fs, process::Command};
use toml_edit::{Document, value};
use chrono::Utc;
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save current explicitly installed packages to archpkglist.toml
    Save,
    /// Restore packages from archpkglist.toml
    Restore,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Save => save()?,
        Commands::Restore => restore()?,
    }
    Ok(())
}

fn save() -> Result<()> {
    // Ensure pacman is available ofc it is available man.
    if Command::new("pacman").arg("--version").output().is_err() {
        eprintln!("{}", "pacman not found. Are you on Arch Linux?".red());
        std::process::exit(1);
    }

    // List explicitly installed packages
    let explicit_out = Command::new("pacman")
        .arg("-Qe")
        .output()
        .context("Failed to list explicitly installed packages")?;
    let explicit_str = String::from_utf8_lossy(&explicit_out.stdout);
    let explicit_pkgs: Vec<&str> = explicit_str
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    // List other (AUR) packages
    let foreign_out = Command::new("pacman")
        .arg("-Qm")
        .output()
        .context("Failed to list foreign packages")?;
    let foreign_str = String::from_utf8_lossy(&foreign_out.stdout);
    let foreign_pkgs: Vec<&str> = foreign_str
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    // Build TOML document
    let mut doc = Document::new();
    doc["timestamp"] = value(Utc::now().to_rfc3339());

    let mut repo_table = toml_edit::Table::new();
    let mut aur_table = toml_edit::Table::new();

    for &pkg in &explicit_pkgs {
        if foreign_pkgs.contains(&pkg) {
            aur_table[pkg] = value(true);
        } else {
            repo_table[pkg] = value(true);
        }
    }

    doc["repo"] = toml_edit::Item::Table(repo_table);
    doc["aur"] = toml_edit::Item::Table(aur_table);

    fs::write("archpkglist.toml", doc.to_string())
        .context("Failed to write archpkglist.toml")?;
    println!("{} archpkglist.toml", "Saved package list to".green());
    Ok(())
}

fn restore() -> Result<()> {
    // Read snapshot file
    let content = fs::read_to_string("archpkglist.toml")
        .context("archpkglist.toml not found. Run 'archsync save' first.")?;
    let doc = content.parse::<Document>()?;

    // Ensure pacman is available
    if Command::new("pacman").arg("--version").output().is_err() {
        eprintln!("{}", "pacman not found. Are you on Arch Linux?".red());
        std::process::exit(1);
    }

    // Only require an AUR helper when the snapshot enables an AUR package.
    let aur = doc.get("aur").and_then(|i| i.as_table());
    let needs_aur_helper = aur.is_some_and(|table| {
        table.iter().any(|(_, val)| val.as_bool().unwrap_or(false))
    });
    let aur_helper = if !needs_aur_helper {
        None
    } else if Command::new("yay").arg("--version").output().is_ok() {
        Some("yay")
    } else if Command::new("paru").arg("--version").output().is_ok() {
        Some("paru")
    } else {
        eprintln!("{}", "Neither 'yay' nor 'paru' found for AUR support.".yellow());
        eprintln!("Install one by: git clone https://aur.archlinux.org/yay.git && cd yay && makepkg -si");
        std::process::exit(1);
    };

    // Install repo packages
    if let Some(repo) = doc.get("repo").and_then(|i| i.as_table()) {
        println!("{}", "Installing repo packages:".blue());
        for (pkg, val) in repo.iter() {
            if val.as_bool().unwrap_or(false) {
                let status = Command::new("sudo")
                    .arg("pacman")
                    .arg("-S")
                    .arg("--needed")
                    .arg(pkg)
                    .status();
                match status {
                    Ok(s) if s.success() => println!("{} {}", "Installed".green(), pkg),
                    _ => eprintln!("{} {}", "Failed".red(), pkg),
                }
            }
        }
    }

    // Install AUR packages
    if let (Some(aur), Some(aur_helper)) = (aur, aur_helper) {
        println!("{} {}", "Installing AUR packages with".blue(), aur_helper);
        for (pkg, val) in aur.iter() {
            if val.as_bool().unwrap_or(false) {
                let status = Command::new(aur_helper)
                    .arg("-S")
                    .arg("--needed")
                    .arg(pkg)
                    .status();
                match status {
                    Ok(s) if s.success() => println!("{} {}", "Installed".green(), pkg),
                    _ => eprintln!("{} {}", "Failed".red(), pkg),
                }
            }
        }
    }

    Ok(())
}
