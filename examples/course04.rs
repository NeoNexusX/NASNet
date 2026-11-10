//! 第 04 课可选示例。将歌单写成 TOML，再观察嵌套 span 和日志过滤。

use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
};

use serde::{Deserialize, Serialize};
use tracing::{debug, debug_span, info, info_span, trace};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Playlist {
    version: u32,
    name: String,
    songs: Vec<Song>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Song {
    title: String,
    artist: Option<String>,
    seconds: u32,
}

fn sample_playlist() -> Playlist {
    Playlist {
        version: 1,
        name: "通勤".into(),
        songs: vec![
            Song {
                title: "晴天".into(),
                artist: Some("周杰伦".into()),
                seconds: 269,
            },
            Song {
                title: "无名小调".into(),
                artist: None,
                seconds: 180,
            },
        ],
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .without_time()
        .init();

    let playlist = sample_playlist();
    let text = toml::to_string_pretty(&playlist)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("playlist.toml");

    let span = info_span!("sync", playlist = %playlist.name);
    let _entered = span.enter();
    {
        let write_span = debug_span!("write_playlist");
        let _entered = write_span.enter();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        debug!(bytes = text.len(), "歌单已写入");
    }

    let saved: Playlist = toml::from_str(&fs::read_to_string(&path)?)?;
    trace!("读取完成");
    info!(songs = saved.songs.len(), "歌单已恢复");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_survives_toml_round_trip() -> Result<(), Box<dyn Error>> {
        let original = sample_playlist();
        let text = toml::to_string_pretty(&original)?;
        assert_eq!(toml::from_str::<Playlist>(&text)?, original);
        Ok(())
    }
}
