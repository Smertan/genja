use genja_core::genja_task;

struct BadTask;

#[genja_task(name = "bad")]
impl BadTask {
    fn helper(&self) {}
}

fn main() {}
