#[derive(Debug)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub len: usize,
    #[allow(dead_code)]
    pub quote: Option<char>,
}

pub fn tokenize(input: &str) -> (Token, Vec<Token>) {
    let input = input.trim();

    if input.is_empty() {
        return (
            Token {
                text: "".into(),
                start: 0,
                len: 0,
                quote: None,
            },
            Vec::new(),
        );
    }

    let idx = input
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(input.len());

    let mut residual = input[idx..].trim();
    let mut output = Vec::new();

    enum Mode {
        Quoted(char),
        Bare,
        Searching,
    }

    let mut mode = Mode::Searching;

    let mut start_idx = idx;

    loop {
        match mode {
            Mode::Quoted(quote) => {
                let local = &residual[1..];
                let mut last = '\0';
                let mut last_idx = 0;
                let mut arg = String::new();
                for (idx, ch) in local.char_indices() {
                    last_idx = idx;
                    if ch == quote && last != '\\' {
                        break;
                    } else if ch == 'n' && last != '\\' {
                        last = ch;
                        arg.push('\n');
                    } else {
                        last = ch;
                        arg.push(ch);
                    }
                }
                output.push(Token {
                    text: arg,
                    start: start_idx,
                    len: last_idx + 2,
                    quote: Some(quote),
                });

                if last_idx + 2 < residual.len() {
                    residual = &residual[last_idx + 2..];
                    mode = Mode::Searching;
                } else {
                    break;
                }
            }
            Mode::Bare => {
                let idx = residual
                    .char_indices()
                    .find(|(_, ch)| ch.is_whitespace())
                    .map(|(idx, _)| idx)
                    .unwrap_or(residual.len());

                output.push(Token {
                    text: residual[..idx].to_string(),
                    start: start_idx,
                    len: idx,
                    quote: None,
                });
                residual = &residual[idx..];
                mode = Mode::Searching
            }
            Mode::Searching => {
                residual = residual.trim_start();
                start_idx = residual.as_ptr() as usize - input.as_ptr() as usize;
                if !residual.is_empty() {
                    mode = match residual.as_bytes()[0] {
                        b'"' => Mode::Quoted('"'),
                        b'\'' => Mode::Quoted('\''),
                        _ => Mode::Bare,
                    }
                }
            }
        }

        if residual.is_empty() {
            break;
        }
    }

    let quote = match mode {
        Mode::Quoted(ch) => Some(ch),
        _ => None,
    };

    (
        Token {
            text: String::from(&input[..idx]),
            start: 0,
            len: idx,
            quote,
        },
        output,
    )
}
