use serde::Deserialize;
use std::io::{Write, stdout};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub mod tmpdir {
    use std::path::PathBuf;

    pub struct TmpDir {
        pub path: PathBuf,
    }

    impl Default for TmpDir {
        fn default() -> Self {
            let path = std::env::temp_dir().join("cursorup_temp");
            Self { path }
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            println!("Cleaning up temporary directory: {:?}", self.path);
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Resp {
    pub version: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "commitSha")]
    pub commit_sha: String,
    #[serde(rename = "rehUrl")]
    pub reh_url: String,
}

async fn fetch_metadata() -> Result<Resp> {
    let url = "https://cursor.com/api/download?platform=linux-x64&releaseTrack=stable";
    let resp = reqwest::get(url).await?.json::<Resp>().await?;
    Ok(resp)
}

async fn download_file(url: &str, dest_path: &Path) -> Result<()> {
    println!("Downloading from {}", url);
    let mut response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()).into());
    }

    let total_size = response
        .content_length()
        .ok_or("Failed to get content length")?;

    let mut file = fs::File::create(dest_path).await?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        let percentage = (downloaded as f64 / total_size as f64) * 100.0;

        print!(
            "\rDownloading... {:.2}% ({:.2}MB / {:.2}MB)",
            percentage,
            downloaded as f64 / 1_048_576.0,
            total_size as f64 / 1_048_576.0
        );
        stdout().flush()?;
    }

    println!();
    println!("Download completed successfully to {:?}", dest_path);

    Ok(())
}

async fn install(
    appimage_path: &Path,
    _version: &str, // version is not used for the destination path anymore
    tmp_dir: &Path,
) -> Result<()> {
    println!("Starting installation...");

    let mut perms = fs::metadata(appimage_path).await?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    fs::set_permissions(appimage_path, perms).await?;
    println!("Granted execute permissions to {:?}", appimage_path);

    // --appimage-extract
    println!("Extracting AppImage...");
    let output = Command::new(appimage_path)
        .arg("--appimage-extract")
        .current_dir(tmp_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!(
            "AppImage extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let extracted_dir = tmp_dir.join("squashfs-root");
    println!("Extracted to {:?}", extracted_dir);

    let home_dir = PathBuf::from(std::env::var("HOME")?);
    let dest_dir = home_dir.join("Applications").join("cursor");
    fs::create_dir_all(&dest_dir).await?;
    println!("Ensured destination directory exists: {:?}", dest_dir);

    back_file(dest_dir.clone()).await?;

    let icon_dest_path = dest_dir.join("code.png");
    let icon_source_path = extracted_dir.join("code.png");
    fs::copy(&icon_source_path, &icon_dest_path).await?;
    println!("Copied icon to {:?}", icon_dest_path);

    let appimage_dest_path = dest_dir.join(appimage_path.file_name().unwrap());
    fs::copy(appimage_path, &appimage_dest_path).await?;
    println!("Copied AppImage to {:?}", appimage_dest_path);
    echo_2_desktop(&appimage_dest_path, &icon_dest_path).await?;
    println!("Installation complete!");
    Ok(())
}

pub async fn echo_2_desktop(appimage_path: &PathBuf, icon_path: &PathBuf) -> Result<()> {
    let contents = format!(
        r#"[Desktop Entry]
Name=Cursor
Exec={}
Icon={}
Type=Application
Categories=Utility;Development;
Terminal=false"#,
        appimage_path.to_str().unwrap(),
        icon_path.to_str().unwrap(),
    );
    let home_dir = PathBuf::from(std::env::var("HOME")?);
    let desktop_path = home_dir.join(".local/share/applications/cursor.desktop");
    fs::write(desktop_path, contents).await?;
    Ok(())
}

pub async fn back_file(dir_path: PathBuf) -> Result<()> {
    let back_dir = dir_path.join("back");
    fs::create_dir_all(&back_dir).await?;

    let mut entries = fs::read_dir(dir_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            if extension == "AppImage" || extension == "png" {
                if let Some(file_name) = path.file_name() {
                    let mut backup_file_name = file_name.to_os_string();
                    backup_file_name.push(".bak");
                    let dest_path = back_dir.join(backup_file_name);

                    println!("Backing up {:?} to {:?}", &path, &dest_path);
                    fs::rename(&path, &dest_path).await?;
                }
            }
        }
    }
    Ok(())
}

pub async fn run() -> Result<()> {
    println!("Starting cursorup process...");

    let metadata = fetch_metadata().await?;
    println!("Successfully fetched metadata: {metadata:#?}");

    let tmp_dir = tmpdir::TmpDir::default();
    fs::create_dir_all(&tmp_dir.path).await?;
    println!("Created temporary directory: {:?}", tmp_dir.path);

    let download_url = &metadata.download_url;
    let file_name = download_url
        .split('/')
        .last()
        .unwrap_or("cursor-download.tmp");
    let appimage_path = tmp_dir.path.join(file_name);
    download_file(download_url, &appimage_path).await?;
    install(&appimage_path, &metadata.version, &tmp_dir.path).await?;
    println!("Cursorup process finished successfully.");
    Ok(())
}
