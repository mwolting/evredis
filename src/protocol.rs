#[derive(Debug)]
pub enum Command {
    Get(String),
}

#[derive(Debug)]
pub enum Response {
    String(String),
}
