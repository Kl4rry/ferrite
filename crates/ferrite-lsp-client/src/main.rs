use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
};
mod types;

use serde::Serialize;
use serde_json::Value;

fn write_lsp_request<Request: types::Request>(writer: &mut impl Write, params: &Request) {
    #[derive(Serialize)]
    struct JsonRpc<Request> {
        jsonrpc: &'static str,
        method: &'static str,
        id: u32,
        params: Request,
    }
    let text = serde_json::to_string(&JsonRpc {
        jsonrpc: "2.0",
        id: 1,
        method: Request::METHOD,
        params,
    })
    .expect("unable to serialize request to json");

    let pretty = serde_json::to_string_pretty(&JsonRpc {
        jsonrpc: "2.0",
        id: 1,
        method: Request::METHOD,
        params,
    })
    .expect("unable to serialize request to json");

    let content_length = text.len();
    let header = format!("Content-Length: {}\r\n\r\n", content_length);

    writer.write_all(header.as_bytes()).unwrap();
    writer.write_all(text.as_bytes()).unwrap();
    writer.flush().unwrap();

    std::io::stdout().write_all(header.as_bytes()).unwrap();
    std::io::stdout().write_all(pretty.as_bytes()).unwrap();
    std::io::stdout().write_all(b"\n").unwrap();
}

fn parse_framing(reader: &mut (impl Read + BufRead)) -> anyhow::Result<String> {
    let mut size = None;
    let mut buf = String::new();
    loop {
        buf.clear();
        if reader.read_line(&mut buf)? == 0 {
            break;
        }
        if !buf.ends_with("\r\n") {
            anyhow::bail!("malformed header: {:?}", buf);
        }
        let buf = &buf[..buf.len() - 2];
        if buf.is_empty() {
            break;
        }
        let mut parts = buf.splitn(2, ": ");
        let header_name = parts.next().unwrap();
        let Some(header_value) = parts.next() else {
            anyhow::bail!("malformed header: {:?}", buf)
        };
        if header_name.eq_ignore_ascii_case("Content-Length") {
            size = Some(header_value.parse::<usize>()?);
        }
    }
    let Some(size) = size else {
        anyhow::bail!("no Content-Length")
    };
    let mut buf = buf.into_bytes();
    buf.resize(size, 0);
    reader.read_exact(&mut buf)?;
    let buf = String::from_utf8(buf)?;
    Ok(buf)
}

fn parse_json_rpc(_text: &str) {
    #[derive(Serialize)]
    struct JsonRpc {
        jsonrpc: &'static str,
        id: u32,
        result: Value,
    }
}

fn main() {
    let cwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut cmd = Command::new("rust-analyzer");
    // let mut cmd = Command::new("clangd");
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped())/*.stderr(Stdio::null())*/;

    let mut child = cmd.spawn().unwrap();
    let mut stdin = std::io::BufWriter::new(child.stdin.take().unwrap());
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    {
        let initialize_request = types::InitializeRequest {
            process_id: Some(std::process::id()),
            root_path: Some(cwd.clone()),
            root_uri: Some(format!("file://{cwd}")),
            client_info: Some(types::ClientInfo {
                name: "ferrite".into(),
                version: None,
            }),
            capabilities: types::ClientCapabilities {
                general: Some(types::GeneralClientCapabilities {
                    position_encodings: Some(vec![String::from("utf-8")]),
                }),
            },
            locale: None,
            work_done_progress_params: types::WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        write_lsp_request(&mut stdin, &initialize_request);

        let raw_msg = parse_framing(&mut stdout).unwrap();
        eprintln!("{raw_msg}");
    }
}
