//! 第 01 课示例练习：结构体与方法、所有权与借用、Vec 与切片、Option、迭代器与稳定排序。
//!
//! 运行：cargo run --locked --example course01
//! 本课已完成，文件中是参考实现。复习时可以删掉某个方法体自己重写，再运行对比。

/// 一条任务拥有自己的字符串；priority 越小越优先。
struct Task {
    id: String,
    title: String,
    priority: u32,
}

/// 清单拥有名称和任务列表。
struct TodoList {
    name: String,
    tasks: Vec<Task>,
}

impl TodoList {
    /// E1.1：关联函数，没有 self。name 的所有权移入新清单。
    fn new(name: String) -> Self {
        Self {
            name,
            tasks: Vec::new(),
        }
    }

    /// E1.2：&self 只读借用；返回的 &str 借自清单。
    fn name(&self) -> &str {
        &self.name
    }

    /// E1.2：&mut self 可以修改；新名称移入，旧名称被丢弃。
    fn rename(&mut self, name: String) {
        self.name = name;
    }

    /// E1.3：按值接收，任务的所有权移入 Vec。
    fn add(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// E1.3：只读切片，调用方可以遍历，但不能追加或删除。
    fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// E1.4：for 循环写法。找到时借出元素，找不到返回 None。
    /// 保留它是为了和下面的迭代器写法对照；clippy 会建议改成 iter().find()。
    #[allow(clippy::manual_find)]
    fn find_by_loop(&self, id: &str) -> Option<&Task> {
        for task in &self.tasks {
            if task.id == id {
                return Some(task);
            }
        }
        None
    }

    /// E1.4：迭代器写法，与 find_by_loop 等价。
    fn find(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// E1.5：收集引用后稳定排序；只改变引用的顺序，保存顺序不变。
    fn by_priority(&self) -> Vec<&Task> {
        let mut tasks: Vec<&Task> = self.tasks.iter().collect();
        tasks.sort_by_key(|task| task.priority);
        tasks
    }
}

fn task(id: &str, title: &str, priority: u32) -> Task {
    Task {
        id: id.to_owned(),
        title: title.to_owned(),
        priority,
    }
}

fn main() {
    // E1.1：name 移入清单后不能再使用。
    let name = String::from("本周");
    let mut list = TodoList::new(name);
    // println!("{name}"); // 取消注释：E0382 borrow of moved value

    // E1.2：只读借用在最后一次使用后结束，之后才能可变借用。
    let current = list.name();
    println!("改名前：{current}");
    list.rename(String::from("本周待办"));
    println!("改名后：{}", list.name());

    // E1.3：Vec 拥有任务，外部通过切片读取。
    list.add(task("backup", "备份照片", 2));
    list.add(task("mount", "挂载共享", 1));
    list.add(task("clean", "清理下载", 2));
    println!("共 {} 条任务", list.tasks().len());

    // E1.4：Option 表达“可能没有”。
    if let Some(found) = list.find("mount") {
        println!("找到：{}", found.title);
    }
    assert_eq!(
        list.find_by_loop("backup").map(|task| task.title.as_str()),
        list.find("backup").map(|task| task.title.as_str())
    );
    assert!(list.find("missing").is_none());

    // E1.5：同优先级保持保存顺序，原列表不变。
    let candidates: Vec<&str> = list
        .by_priority()
        .iter()
        .map(|task| task.id.as_str())
        .collect();
    let saved: Vec<&str> = list.tasks().iter().map(|task| task.id.as_str()).collect();
    println!("候选顺序：{candidates:?}，保存顺序：{saved:?}");
    assert_eq!(candidates, ["mount", "backup", "clean"]);
    assert_eq!(saved, ["backup", "mount", "clean"]);
}
