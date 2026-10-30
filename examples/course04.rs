//! 第 04 课示例练习：Serde 与 TOML、自定义错误、路径、文件读写、tracing 与测试。
//!
//! 运行：cargo run --locked --example course04
//! 测试：cargo test --locked --example course04（E4.6）
//!
//! 按 E4.1 → E4.5 的顺序把 todo!() 换成实现。main 依次检查每道题，
//! 遇到未完成的题会停在对应的 todo!() 并显示题号。

use std::{
    error::Error,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

fn main() -> Result<(), Box<dyn Error>> {
    check_toml()?; // E4.1
    check_errors()?; // E4.2
    check_paths(); // E4.3
    check_files()?; // E4.4
    trace_nested_spans(); // E4.5
    println!("第 04 课示例练习完成；E4.6 用 cargo test --locked --example course04 检查");
    Ok(())
}

// ---------- E4.1 Serde 与 TOML ----------

/// 练习用的歌单：顶层字段、表数组和可选字段。
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
        name: "通勤".to_owned(),
        songs: vec![
            Song {
                title: "晴天".to_owned(),
                artist: Some("周杰伦".to_owned()),
                seconds: 269,
            },
            Song {
                title: "无名小调".to_owned(),
                artist: None,
                seconds: 180,
            },
        ],
    }
}

/// TODO E4.1：用 toml::to_string_pretty 编码。
fn encode_playlist(playlist: &Playlist) -> Result<String, toml::ser::Error> {
    let _ = playlist;
    todo!("E4.1: encode a playlist as TOML")
}

/// TODO E4.1：用 toml::from_str 解码；目标类型由返回值决定。
fn decode_playlist(text: &str) -> Result<Playlist, toml::de::Error> {
    let _ = text;
    todo!("E4.1: decode a playlist from TOML")
}

fn check_toml() -> Result<(), Box<dyn Error>> {
    let playlist = sample_playlist();
    let text = encode_playlist(&playlist)?;
    println!("E4.1 编码结果：\n{text}");
    assert!(
        text.contains("[[songs]]"),
        "Vec<Song> 应编码为 [[songs]] 表数组"
    );
    assert_eq!(
        text.matches("artist").count(),
        1,
        "值为 None 的字段应被省略"
    );
    assert_eq!(decode_playlist(&text)?, playlist, "编码后再解码应得到原值");
    let unknown = "version = 1\nname = \"x\"\nsongs = []\nrating = 5\n";
    assert!(
        decode_playlist(unknown).is_err(),
        "deny_unknown_fields 应拒绝多余字段"
    );
    let missing = "name = \"x\"\nsongs = []\n";
    assert!(decode_playlist(missing).is_err(), "缺少必需字段应解析失败");
    println!("E4.1 完成");
    Ok(())
}

// ---------- E4.2 自定义错误 ----------

/// 加载歌单可能出现的三种失败。
#[derive(Debug)]
enum PlaylistError {
    Toml(toml::de::Error),
    UnsupportedVersion(u32),
    EmptyName,
}

impl fmt::Display for PlaylistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO E4.2：用 match 为每个变体写一句英文摘要。
        // Toml 变体不要拼接底层错误：它的内容会复述原始输入。
        let _ = formatter;
        todo!("E4.2: Display for PlaylistError")
    }
}

impl Error for PlaylistError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // TODO E4.2：Toml 变体借出底层错误，其他变体返回 None。
        todo!("E4.2: Error::source for PlaylistError")
    }
}

impl From<toml::de::Error> for PlaylistError {
    fn from(error: toml::de::Error) -> Self {
        // TODO E4.2：包装成 Toml 变体，让 ? 能自动转换。
        let _ = error;
        todo!("E4.2: From<toml::de::Error> for PlaylistError")
    }
}

/// TODO E4.2：解码（用 ? 转换错误）→ version 必须为 1 → name 不能只有空白。
fn load_playlist(text: &str) -> Result<Playlist, PlaylistError> {
    let _ = text;
    todo!("E4.2: decode and validate a playlist")
}

fn check_errors() -> Result<(), Box<dyn Error>> {
    let parse_error = decode_playlist("SECRET = [").expect_err("非法 TOML 必须解析失败");
    println!("E4.2 toml 原始错误会复述输入：\n{parse_error}");
    for error in [
        PlaylistError::Toml(parse_error),
        PlaylistError::UnsupportedVersion(2),
        PlaylistError::EmptyName,
    ] {
        println!("E4.2 错误摘要：{error}");
        assert!(
            !error.to_string().contains("SECRET"),
            "Display 不应复述原始输入"
        );
        match &error {
            PlaylistError::Toml(inner) => {
                let source = error.source().expect("Toml 变体应借出底层错误");
                assert_eq!(
                    source.to_string(),
                    inner.to_string(),
                    "source 应是被包装的那个错误"
                );
            }
            _ => assert!(error.source().is_none(), "其他变体没有 source"),
        }
    }
    let text = encode_playlist(&sample_playlist())?;
    assert_eq!(load_playlist(&text)?.name, "通勤");
    assert!(matches!(
        load_playlist("SECRET = ["),
        Err(PlaylistError::Toml(_))
    ));
    let old = text.replace("version = 1", "version = 2");
    assert!(matches!(
        load_playlist(&old),
        Err(PlaylistError::UnsupportedVersion(2))
    ));
    let blank = text.replace("name = \"通勤\"", "name = \" \"");
    assert!(matches!(
        load_playlist(&blank),
        Err(PlaylistError::EmptyName)
    ));
    println!("E4.2 完成");
    Ok(())
}

// ---------- E4.3 路径 ----------

/// TODO E4.3：relative 必须非空、是相对路径、扩展名为 toml，且每一段都是 Component::Normal；
/// 满足时返回 base_file 所在目录下的完整路径，否则返回 None。只检查形式，不访问磁盘。
fn resolve_beside(base_file: &Path, relative: &Path) -> Option<PathBuf> {
    let _ = (base_file, relative);
    todo!("E4.3: resolve a relative TOML path beside a base file")
}

fn check_paths() {
    let base = Path::new("/srv/nasnet/config.toml");
    println!("E4.3 {base:?} 的 parent：{:?}", base.parent());
    println!(
        "E4.3 只有文件名时 parent 是空路径：{:?}",
        Path::new("config.toml").parent()
    );
    for sample in ["devices/home.toml", "../home.toml", "/etc/home.toml"] {
        let parts: Vec<Component> = Path::new(sample).components().collect();
        println!("E4.3 {sample} 拆分为 {parts:?}");
    }
    assert_eq!(
        resolve_beside(base, Path::new("devices/home.toml")),
        Some(PathBuf::from("/srv/nasnet/devices/home.toml"))
    );
    for bad in [
        "",
        "../home.toml",
        "./home.toml",
        "/etc/home.toml",
        "devices/home.txt",
        "devices/",
    ] {
        assert_eq!(resolve_beside(base, Path::new(bad)), None, "应拒绝 {bad:?}");
    }
    println!("E4.3 完成");
}

// ---------- E4.4 文件读写 ----------

/// TODO E4.4：读取整个文本文件；文件不存在时返回 Ok(None)，其他错误原样返回。
fn read_optional(path: &Path) -> io::Result<Option<String>> {
    let _ = path;
    todo!("E4.4: read a text file that may be missing")
}

/// TODO E4.4：用 OpenOptions 的 create_new 创建新文件再 write_all；目标已存在时返回错误，不覆盖。
/// write_all 是 std::io::Write trait 的方法，需要先 use 这个 trait。
fn write_new(path: &Path, contents: &str) -> io::Result<()> {
    let _ = (path, contents);
    todo!("E4.4: create a new file without overwriting")
}

fn check_files() -> Result<(), Box<dyn Error>> {
    // TempDir 离开作用域时删除整个目录，练习不会在项目中留下文件。
    let directory = tempfile::tempdir()?;
    let note = directory.path().join("note.txt");
    assert_eq!(read_optional(&note)?, None, "不存在的文件应得到 None");
    write_new(&note, "第一版")?;
    assert_eq!(read_optional(&note)?.as_deref(), Some("第一版"));
    let error = write_new(&note, "第二版").expect_err("create_new 不能覆盖已有文件");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert!(
        read_optional(directory.path()).is_err(),
        "读取目录失败不是“文件不存在”"
    );

    // 演示 rename：先写好同目录的新文件，再整体替换旧文件。P4.6 原子替换的最后一步就是它。
    let draft = directory.path().join("note.txt.tmp");
    write_new(&draft, "第二版")?;
    fs::rename(&draft, &note)?;
    assert_eq!(read_optional(&note)?.as_deref(), Some("第二版"));
    assert_eq!(
        fs::read_dir(directory.path())?.count(),
        1,
        "rename 后不应留下临时文件"
    );
    println!("E4.4 完成");
    Ok(())
}

// ---------- E4.5 tracing ----------

/// TODO E4.5：
/// 1. 用 tracing_subscriber::fmt() 安装 subscriber，最高级别 DEBUG（with_max_level）；
/// 2. 创建 info_span!("sync", playlist = "通勤") 并 enter，记录一条 info 事件；
/// 3. 在内层代码块中创建 debug_span!("song", title = "晴天") 并 enter，记录 debug!(seconds = 269, "copied")；
/// 4. 代码块结束后记录一条 trace 事件和一条 info 事件。
///
/// 运行后对照输出：哪条事件没有出现？最后一条 info 属于哪个 span？
fn trace_nested_spans() {
    todo!("E4.5: install a subscriber and record events in nested spans")
}

// ---------- E4.6 测试 ----------
//
// TODO E4.6：在下面添加 #[cfg(test)] mod tests（用 use super::*; 引入本文件的条目），写三个 #[test]：
// 1. 只有一首歌、artist 为 None 的歌单，编码后再解码得到原值；
// 2. resolve_beside(Path::new("config.toml"), Path::new("devices/a.toml")) 返回相对路径 devices/a.toml；
// 3. 在 tempfile::tempdir() 中先 write_new 再 read_optional，读回相同的内容。
// 运行：cargo test --locked --example course04
