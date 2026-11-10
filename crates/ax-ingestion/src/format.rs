use crate::{IngestionError, IngestionResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    Json,
    Jsonl,
    Csv,
    Txt,
    Pdf,
    Docx,
    Image,
    Audio,
    Video,
}

impl DataFormat {
    pub fn from_filename(filename: &str) -> IngestionResult<Self> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or(IngestionError::UnsupportedFormat)?;
        match extension.as_str() {
            "json" => Ok(Self::Json),
            "jsonl" | "ndjson" => Ok(Self::Jsonl),
            "csv" => Ok(Self::Csv),
            "txt" | "md" => Ok(Self::Txt),
            "pdf" => Ok(Self::Pdf),
            "docx" => Ok(Self::Docx),
            "png" | "jpg" | "jpeg" | "webp" => Ok(Self::Image),
            "mp3" | "wav" | "ogg" | "m4a" | "flac" => Ok(Self::Audio),
            "mp4" | "mov" | "mkv" | "webm" => Ok(Self::Video),
            _ => Err(IngestionError::UnsupportedFormat),
        }
    }

    pub fn validate_payload(self, bytes: &[u8]) -> IngestionResult<()> {
        let valid = match self {
            Self::Json => serde_json::from_slice::<serde_json::Value>(bytes).is_ok(),
            Self::Jsonl => {
                std::str::from_utf8(bytes).is_ok()
                    && bytes.split(|byte| *byte == b'\n')
                        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
                        .all(|line| serde_json::from_slice::<serde_json::Value>(line).is_ok())
            }
            Self::Csv => std::str::from_utf8(bytes).is_ok()
                && bytes.contains(&b',')
                && bytes.iter().any(|byte| *byte == b'\n'),
            Self::Txt => std::str::from_utf8(bytes).is_ok(),
            Self::Pdf => bytes.starts_with(b"%PDF-"),
            Self::Docx => bytes.starts_with(b"PK\x03\x04"),
            Self::Image => {
                bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                    || bytes.starts_with(b"\xff\xd8\xff")
                    || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
            }
            Self::Audio | Self::Video => !bytes.is_empty(),
        };
        if valid { Ok(()) } else { Err(IngestionError::InvalidPayload) }
    }
}
