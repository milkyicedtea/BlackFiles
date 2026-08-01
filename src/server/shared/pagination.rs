#[derive(Clone, Copy)]
pub(crate) struct Page {
    pub(crate) number: i64,
    pub(crate) limit: i64,
    pub(crate) offset: i64,
}

impl Page {
    pub(crate) fn new(number: Option<i64>, limit: Option<i64>) -> Self {
        let number = number.unwrap_or(1).max(1);
        let limit = limit.unwrap_or(50).clamp(1, 200);
        Self {
            number,
            limit,
            offset: (number - 1) * limit,
        }
    }
}
