//! 阻塞 HTTP/1.1 客户端(Std API: http_request,零框架依赖,ureq 传输)
//! 语义见 docs/STD_API_SPEC.md §2:
//! - 支持 http/https; 返回 (status, 响应体流)
//! - 调用方逐行读取(含空行/SSE 心跳), 每行回调一次
//! - 网络/协议错误 → Err(由调用方映射为 status 0)

pub struct Resp {
    pub status: i32,
    pub reader: Box<dyn std::io::BufRead>,
}

pub fn open(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &str,
) -> Result<Resp, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(300))
        .build();
    let method_up = method.to_uppercase();
    let mut req = match method_up.as_str() {
        "GET" => agent.get(url),
        "POST" => agent.post(url),
        "PUT" => agent.put(url),
        "DELETE" => agent.delete(url),
        "HEAD" => agent.head(url),
        "PATCH" => agent.patch(url),
        other => return Err(format!("http_request: 不支持的方法 {}", other)),
    };
    for (k, v) in headers {
        if !k.is_empty() && !k.eq_ignore_ascii_case("host") && !k.eq_ignore_ascii_case("content-length") {
            req = req.set(k, v);
        }
    }
    let resp = if method_up == "GET" || method_up == "HEAD" {
        req.call().map_err(|e| http_err(&e))?
    } else {
        req.send_string(body).map_err(|e| http_err(&e))?
    };
    let status = resp.status() as i32;
    let reader: Box<dyn std::io::BufRead> = Box::new(std::io::BufReader::new(
        resp.into_reader(),
    ));
    Ok(Resp { status, reader })
}

fn http_err(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            format!("http_request: HTTP {} {}", code, resp.status_text())
        }
        other => format!("http_request: {}", other),
    }
}
