pub fn tokenize(input: &str) -> (String, Vec<String>) {
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
                    } else {
                        last = ch;
                        arg.push(ch);
                    }
                }
                output.push(arg);
                residual = &residual[last_idx + 2..];
                mode = Mode::Searching;
            }
            Mode::Bare => {
                let idx = residual
                    .char_indices()
                    .find(|(_, ch)| ch.is_whitespace())
                    .map(|(idx, _)| idx)
                    .unwrap_or(residual.len());

                output.push(residual[..idx].to_string());
                residual = &residual[idx..];
                mode = Mode::Searching
            }
            Mode::Searching => {
                residual = residual.trim_start();
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

    (String::from(&input[..idx]), output)
}
