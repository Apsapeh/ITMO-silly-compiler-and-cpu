use crate::error;

#[derive(Debug, Clone)]
pub struct SourceLine {
    pub line_number: usize,
    pub power: usize,
    pub string: String,
}

#[derive(Debug, Clone)]
pub struct SourceLineWords {
    pub source_line: SourceLine,
    pub words: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LexResult {
    pub string_register: Vec<String>,
    pub lines: Vec<SourceLineWords>,
}

pub fn lex(src: String) -> LexResult {
    let splitted = split_by_line(src);
    check_is_ascii(&splitted);
    let (lines, string_register) = collapse_strings(splitted);
    let lines = separate_key_symb(lines);
    let lines = split_lines_to_words(lines);

    LexResult {
        string_register,
        lines,
    }
}

fn split_by_line(src: String) -> Vec<SourceLine> {
    let mut result = vec![];

    for (i, mut line) in src.lines().enumerate() {
        // Remove comments
        let mut in_string = false;
        if let Some(pos) = line.find(|ch| {
            if ch == '"' {
                in_string = !in_string;
            }
            ch == '!' && !in_string
        }) {
            line = &line[..pos];
        }

        // calculate leading spaces (depth of line)
        let power = line
            .chars()
            .map_while(|c| if c == ' ' { Some(c) } else { None })
            .count();

        // Remove leading and tailing spaces
        line = line.trim();

        if !line.is_empty() {
            result.push(SourceLine {
                line_number: i + 1,
                power,
                string: String::from(line),
            });
        }
    }

    result
}

fn check_is_ascii(lines: &Vec<SourceLine>) {
    for line in lines {
        if !line.string.is_ascii() {
            error::error("Source code and strings MUST BE ASCII", line);
        }
    }
}

// Collapse all static strings to special variables and store them into string register
// "Aboba" -> "0      String register: [0: "Aboba"]
// "67"    -> "1      String register: [0: "Aboba", 1: "67"]
// ...
fn collapse_strings(mut lines: Vec<SourceLine>) -> (Vec<SourceLine>, Vec<String>) {
    let mut result_strings = vec![];

    for line in &mut lines {
        let mut new_line = String::new();

        let mut in_string = false;
        let mut tmp_string = String::new();
        for ch in line.string.chars() {
            if ch == '"' {
                if in_string && !tmp_string.is_empty() {
                    new_line += &format!(" \"{} ", result_strings.len());
                    result_strings.push(tmp_string);
                    tmp_string = String::new();
                }

                in_string = !in_string;
                continue;
            }

            if in_string {
                tmp_string.push(ch);
            } else {
                new_line.push(ch);
            }
        }

        line.string = new_line;
    }

    (lines, result_strings)
}

fn separate_key_symb(mut lines: Vec<SourceLine>) -> Vec<SourceLine> {
    let symbols = [",", "(", ")", "[", "]"];

    for line in &mut lines {
        for symb in symbols {
            line.string = line.string.replace(symb, format!(" {} ", symb).as_str());
        }
    }

    lines
}

fn split_lines_to_words(lines: Vec<SourceLine>) -> Vec<SourceLineWords> {
    lines
        .iter()
        .map(|sl| SourceLineWords {
            source_line: sl.clone(),
            words: sl
                .string
                .split_whitespace()
                .map(|s| s.to_uppercase())
                .collect(),
        })
        .collect()
}
