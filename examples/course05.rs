//! 第 05 课可选实验。用 `?Sized` 让同一个函数接收具体类型和 trait 对象。

trait Speaker {
    fn name(&self) -> &str;

    fn announce(&self, title: &str) -> String {
        format!("{} 正在播放 {title}", self.name())
    }

    fn announce_all<T: AsRef<str>>(&self, titles: &[T]) -> Vec<String>
    where
        Self: Sized,
    {
        titles
            .iter()
            .map(|title| self.announce(title.as_ref()))
            .collect()
    }
}

struct Headphones;

impl Speaker for Headphones {
    fn name(&self) -> &str {
        "耳机"
    }
}

fn describe(speaker: &(impl Speaker + ?Sized)) -> String {
    format!("设备：{}", speaker.name())
}

fn main() {
    let headphones = Headphones;
    let speaker: &dyn Speaker = &headphones;
    let description = describe(speaker);
    assert_eq!(description, describe(&headphones));
    println!("{description}");
    println!("{:?}", headphones.announce_all(&["晴天", "夜曲"]));
}
