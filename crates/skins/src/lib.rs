use std::fs;
use std::path::{Path, PathBuf};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::GenericImageView;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SkinError {
    #[error("Invalid image format or decoding failed: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Invalid skin dimensions: {0}x{1}. Expected 64x64 or 64x32")]
    InvalidDimensions(u32, u32),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No skins available in directory")]
    NoSkinsAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkinModel {
    Classic, // Steve (4px wide arm)
    Slim,    // Alex (3px wide arm)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinInfo {
    pub name: String,
    pub path: PathBuf,
    pub model: SkinModel,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct SkinManager {
    skins_dir: PathBuf,
}

impl SkinManager {
    pub fn new(skins_dir: impl Into<PathBuf>) -> Self {
        let dir = skins_dir.into();
        let _ = fs::create_dir_all(&dir);
        Self { skins_dir: dir }
    }

    /// Validates a skin PNG file and detects whether it is Classic (Steve) or Slim (Alex).
    pub fn inspect_skin_file(path: &Path) -> Result<SkinInfo, SkinError> {
        let img = image::open(path)?;
        let (width, height) = img.dimensions();

        if width != 64 || (height != 64 && height != 32) {
            return Err(SkinError::InvalidDimensions(width, height));
        }

        let model = if height == 32 {
            SkinModel::Classic
        } else {
            // Check transparency of the right-most arm pixel in 64x64 skin
            // In Alex (slim) skins, pixel (54, 20) is transparent, whereas in Steve (classic) it is solid.
            let pixel = img.get_pixel(54, 20);
            if pixel[3] == 0 {
                SkinModel::Slim
            } else {
                SkinModel::Classic
            }
        };

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(SkinInfo {
            name,
            path: path.to_path_buf(),
            model,
            width,
            height,
        })
    }

    /// Lists all valid skin files in the directory.
    pub fn list_skins(&self) -> Vec<SkinInfo> {
        let mut skins = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.skins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("png") {
                    if let Ok(info) = Self::inspect_skin_file(&path) {
                        skins.push(info);
                    }
                }
            }
        }
        skins
    }

    /// Selects a random skin from the directory.
    pub fn get_random_skin(&self) -> Result<SkinInfo, SkinError> {
        let skins = self.list_skins();
        let mut rng = rand::thread_rng();
        skins.choose(&mut rng).cloned().ok_or(SkinError::NoSkinsAvailable)
    }

    /// Builds the Mojang Base64 texture value payload for GameProfile property synchronization.
    pub fn generate_texture_value(
        profile_id: &Uuid,
        profile_name: &str,
        skin_url: &str,
        model: SkinModel,
    ) -> String {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let model_str = match model {
            SkinModel::Classic => "default",
            SkinModel::Slim => "slim",
        };

        let json = serde_json::json!({
            "timestamp": now_ms,
            "profileId": profile_id.simple().to_string(),
            "profileName": profile_name,
            "textures": {
                "SKIN": {
                    "url": skin_url,
                    "metadata": {
                        "model": model_str
                    }
                }
            }
        });

        BASE64.encode(json.to_string().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_model_detection() {
        let steve_path = Path::new("../../skins/steve.png");
        let alex_path = Path::new("../../skins/alex.png");
        if steve_path.exists() && alex_path.exists() {
            let steve_info = SkinManager::inspect_skin_file(steve_path).unwrap();
            let alex_info = SkinManager::inspect_skin_file(alex_path).unwrap();
            assert_eq!(steve_info.model, SkinModel::Classic);
            assert_eq!(alex_info.model, SkinModel::Slim);
        }
    }
}
