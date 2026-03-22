//#[cfg_attr(debug_assertions, derive(Debug))]

#[derive(Clone, Debug, PartialEq)]
pub struct NumWord<'a> {
    pub word: &'a str,
    pub line: usize,
    pub col: usize,
}

impl<'a> NumWord<'a> {
    pub fn new(word: &'a str, line: usize, col: usize) -> NumWord<'a> {
        NumWord { word, line, col }
    }
}
