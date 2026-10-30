//! 第 02 课示例练习：枚举与 match、Result 与 ?、自定义错误、Option 字段、先校验后修改。
//!
//! 运行：cargo run --locked --example course02
//! 本课已完成，文件中是参考实现。复习时可以删掉某个函数体自己重写，再运行对比。

use std::fmt;

/// E2.1：用枚举列出每一种失败，调用方可以精确匹配。
#[derive(Debug)]
enum TaskError {
    EmptyId,
    InvalidDueDay,
    InvalidEmail,
    InvalidPhone,
    DuplicateId(String),
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(formatter, "Task ID must not be empty"),
            Self::InvalidDueDay => write!(formatter, "Due day must be between 1 and 31"),
            Self::InvalidEmail => write!(formatter, "Email address must contain @"),
            Self::InvalidPhone => write!(formatter, "Phone number must contain digits only"),
            Self::DuplicateId(id) => write!(formatter, "Duplicate task ID: {id}"),
        }
    }
}

impl std::error::Error for TaskError {}

/// E2.2：每个变体携带自己的数据，一条提醒恰好是其中一种。
enum Reminder {
    Email(String),
    Sms(String),
}

impl Reminder {
    /// match 按变体分派到各自的检查。
    fn validate(&self) -> Result<(), TaskError> {
        match self {
            Self::Email(address) => {
                if address.contains('@') {
                    Ok(())
                } else {
                    Err(TaskError::InvalidEmail)
                }
            }
            Self::Sms(phone) => {
                if !phone.is_empty() && phone.chars().all(|c| c.is_ascii_digit()) {
                    Ok(())
                } else {
                    Err(TaskError::InvalidPhone)
                }
            }
        }
    }
}

struct Task {
    id: String,
    title: String,
    /// E2.4：None 表示没有截止日；Some(0) 或大于 31 是写错的值。
    due_day: Option<u32>,
    reminder: Reminder,
}

impl Task {
    /// E2.3：按 ID → 截止日 → 提醒方式的顺序检查，遇到第一个错误就返回。
    fn validate(&self) -> Result<(), TaskError> {
        if self.id.trim().is_empty() {
            return Err(TaskError::EmptyId);
        }
        if let Some(day) = self.due_day {
            if !(1..=31).contains(&day) {
                return Err(TaskError::InvalidDueDay);
            }
        }
        self.reminder.validate()?;
        Ok(())
    }
}

/// E2.4：空字符串表示“未指定”；? 遇到解析失败时提前返回。
fn parse_due_day(text: &str) -> Result<Option<u32>, TaskError> {
    if text.is_empty() {
        return Ok(None);
    }
    let day = text.parse::<u32>().map_err(|_| TaskError::InvalidDueDay)?;
    Ok(Some(day))
}

/// E2.5：校验 → 查重 → 修改。任何一步失败都不改变列表。
fn try_add(tasks: &mut Vec<Task>, task: Task) -> Result<(), TaskError> {
    task.validate()?;
    if tasks.iter().any(|saved| saved.id == task.id) {
        return Err(TaskError::DuplicateId(task.id));
    }
    tasks.push(task);
    Ok(())
}

fn task(id: &str, title: &str, due_day: Option<u32>, reminder: Reminder) -> Task {
    Task {
        id: id.to_owned(),
        title: title.to_owned(),
        due_day,
        reminder,
    }
}

fn main() -> Result<(), TaskError> {
    let mut tasks = Vec::new();
    let email = Reminder::Email("me@example.com".to_owned());
    try_add(
        &mut tasks,
        task("backup", "备份照片", parse_due_day("15")?, email),
    )?;
    let sms = Reminder::Sms("13800000000".to_owned());
    try_add(
        &mut tasks,
        task("mount", "挂载共享", parse_due_day("")?, sms),
    )?;

    // E2.3：ID 和提醒方式同时错误时，先检查的 ID 决定结果。
    let invalid = task(" ", "无效", None, Reminder::Email("no-at-sign".to_owned()));
    assert!(matches!(
        try_add(&mut tasks, invalid),
        Err(TaskError::EmptyId)
    ));

    // E2.4：None 合法，Some(0) 和无法解析的文本都不合法。
    assert!(matches!(
        parse_due_day("abc"),
        Err(TaskError::InvalidDueDay)
    ));
    let zero = task("zero", "零号", Some(0), Reminder::Sms("1".to_owned()));
    assert!(matches!(zero.validate(), Err(TaskError::InvalidDueDay)));

    // E2.5：重复 ID 被拒绝，列表保持原样。
    let duplicate = task("backup", "另一个备份", None, Reminder::Sms("1".to_owned()));
    match try_add(&mut tasks, duplicate) {
        Ok(()) => unreachable!("重复 ID 不能添加成功"),
        Err(error) => println!("拒绝：{error}"),
    }
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].title, "备份照片");

    for task in &tasks {
        let due = match task.due_day {
            Some(day) => format!("{day} 日"),
            None => "未指定".to_owned(),
        };
        println!("{}：{}，截止 {due}", task.id, task.title);
    }
    Ok(())
}
