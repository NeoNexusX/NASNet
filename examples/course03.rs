//! 第 03 课示例练习：私有字段、原位置更新、删除交还所有权、生命周期标注、From/TryFrom 快照。
//!
//! 运行：cargo run --locked --example course03
//! 本课已完成，文件中是参考实现。复习时可以删掉某个方法体自己重写，再运行对比。

mod todo {
    use std::fmt;

    #[derive(Debug)]
    pub enum TodoError {
        EmptyId,
        DuplicateId(String),
        NotFound(String),
    }

    impl fmt::Display for TodoError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::EmptyId => write!(formatter, "Task ID must not be empty"),
                Self::DuplicateId(id) => write!(formatter, "Duplicate task ID: {id}"),
                Self::NotFound(id) => write!(formatter, "Task not found: {id}"),
            }
        }
    }

    impl std::error::Error for TodoError {}

    /// 任务是可以公开构造的候选值；保存进清单时才校验。
    #[derive(Clone)]
    pub struct Task {
        pub id: String,
        pub title: String,
    }

    /// E3.1：字段私有，模块外只能通过方法读取和修改，校验规则不会被绕过。
    pub struct TodoList {
        name: String,
        tasks: Vec<Task>,
    }

    impl TodoList {
        pub fn new(name: String) -> Self {
            Self {
                name,
                tasks: Vec::new(),
            }
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn tasks(&self) -> &[Task] {
            &self.tasks
        }

        pub fn try_add(&mut self, task: Task) -> Result<(), TodoError> {
            if task.id.trim().is_empty() {
                return Err(TodoError::EmptyId);
            }
            if self.find(&task.id).is_some() {
                return Err(TodoError::DuplicateId(task.id));
            }
            self.tasks.push(task);
            Ok(())
        }

        /// E3.2：先校验，再用 iter_mut 借出可变引用，通过 *existing 原位置整体替换。
        pub fn try_update(&mut self, task: Task) -> Result<(), TodoError> {
            if task.id.trim().is_empty() {
                return Err(TodoError::EmptyId);
            }
            match self.tasks.iter_mut().find(|saved| saved.id == task.id) {
                Some(existing) => {
                    *existing = task;
                    Ok(())
                }
                None => Err(TodoError::NotFound(task.id)),
            }
        }

        /// E3.3：移出并交还拥有的 Task；Vec::remove 保持其余元素的顺序。
        /// 借来的 id 要 to_owned 后才能放进错误返回。
        pub fn remove(&mut self, id: &str) -> Result<Task, TodoError> {
            let index = self
                .tasks
                .iter()
                .position(|saved| saved.id == id)
                .ok_or_else(|| TodoError::NotFound(id.to_owned()))?;
            Ok(self.tasks.remove(index))
        }

        /// 有 &self 时，返回的引用按省略规则借自 self，与 id 活多久无关。
        pub fn find(&self, id: &str) -> Option<&Task> {
            self.tasks.iter().find(|saved| saved.id == id)
        }
    }

    /// E3.5：公开字段的快照，只是候选数据，不保证合法。
    pub struct TodoData {
        pub name: String,
        pub tasks: Vec<Task>,
    }

    /// 从借用复制出独立的数据：to_owned 复制字符串，to_vec 逐个 clone 元素。
    impl From<&TodoList> for TodoData {
        fn from(list: &TodoList) -> Self {
            Self {
                name: list.name.to_owned(),
                tasks: list.tasks.to_vec(),
            }
        }
    }

    /// 消耗快照，按原顺序逐个 try_add，复用同一套校验；第一处错误即返回，不交出半成品。
    impl TryFrom<TodoData> for TodoList {
        type Error = TodoError;

        fn try_from(data: TodoData) -> Result<Self, Self::Error> {
            let mut list = TodoList::new(data.name);
            for task in data.tasks {
                list.try_add(task)?;
            }
            Ok(list)
        }
    }
}

use todo::{Task, TodoData, TodoError, TodoList};

/// E3.4：两个参数都是引用、又没有 self，编译器无法推断返回值借自谁，必须标注。
/// 'a 声明返回值只借自 list，与 id 无关。
fn find_title<'a>(list: &'a TodoList, id: &str) -> Option<&'a str> {
    list.find(id).map(|task| task.title.as_str())
}

fn task(id: &str, title: &str) -> Task {
    Task {
        id: id.to_owned(),
        title: title.to_owned(),
    }
}

fn ids(list: &TodoList) -> Vec<&str> {
    list.tasks().iter().map(|task| task.id.as_str()).collect()
}

fn main() -> Result<(), TodoError> {
    let mut list = TodoList::new("本周".to_owned());
    for (id, title) in [
        ("backup", "备份照片"),
        ("mount", "挂载共享"),
        ("clean", "清理下载"),
    ] {
        list.try_add(task(id, title))?;
    }
    // E3.1：取消注释会编译失败，tasks 是 todo 模块的私有字段。
    // list.tasks.push(task("skip", "绕过校验"));

    // E3.2：原位置更新；不存在的 ID 返回错误，列表不变。
    list.try_update(task("mount", "挂载两个共享"))?;
    assert!(matches!(
        list.try_update(task("missing", "不存在")),
        Err(TodoError::NotFound(_))
    ));
    assert_eq!(ids(&list), ["backup", "mount", "clean"]);

    // E3.3：删除交还所有权，其余顺序不变；重复删除返回错误。
    let removed = list.remove("backup")?;
    println!("删除后取得任务：{}（{}）", removed.title, removed.id);
    assert_eq!(ids(&list), ["mount", "clean"]);
    assert!(matches!(list.remove("backup"), Err(TodoError::NotFound(_))));

    // E3.4：查询字符串先离开作用域，借到的标题仍然可用。
    let title = {
        let query = String::from("mount");
        find_title(&list, &query)
    };
    println!("查询字符串释放后仍可读取：{}", title.unwrap_or("无"));

    // E3.5：快照独立于原清单；恢复时重新校验。
    let mut snapshot = TodoData::from(&list);
    snapshot.name.push_str("（副本）");
    let copy = TodoList::try_from(snapshot)?;
    println!("原清单：{}；恢复的副本：{}", list.name(), copy.name());
    assert_eq!(list.name(), "本周");

    let mut broken = TodoData::from(&list);
    broken.tasks.push(task("mount", "重复 ID"));
    match TodoList::try_from(broken) {
        Ok(_) => unreachable!("重复 ID 的快照不能恢复"),
        Err(error) => println!("恢复失败：{error}"),
    }
    Ok(())
}
