# Cursor AppImage Updater for Linux

This is a simple Rust utility to automatically update the [Cursor](https://cursor.sh/) AppImage on Linux.

The script checks for the latest release of Cursor from its official repository, compares it with the local version specified in `config.toml`, and if a newer version is available, it downloads and replaces the existing AppImage.

## Features

- Checks for the latest Cursor release via the GitHub API.
- Parses version information from your local AppImage file name.
- Downloads the new AppImage if an update is available.
- Replaces the old AppImage and makes the new one executable.
- Updates the corresponding `.desktop` file to point to the new AppImage.

## Prerequisites

Before running, you need to create a `config.toml` file in the project root with the following content:

```toml
# Path to your current Cursor AppImage
app_image_path = "/path/to/your/Cursor-x.x.x-x86_64.AppImage"

# Path to the .desktop file for Cursor
desktop_file_path = "/path/to/your/cursor.desktop"
```

Replace the paths with the actual locations on your system.

## Usage

To run the updater, execute the following command from the project's root directory:

```bash
cargo run
```

The program will handle the check, download, and replacement process automatically.
