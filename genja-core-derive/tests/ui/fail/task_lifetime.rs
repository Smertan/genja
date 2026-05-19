use genja_core_derive::Task;

#[derive(Task)]
struct LifetimeTask<'a> {
    name: &'a str,
}

fn main() {}
