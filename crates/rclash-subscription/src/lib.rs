use anyhow::{anyhow, Context};
use percent_encoding::percent_decode_str;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Yaml,
    Base64,
    Text,
}

pub fn detect_format(input: &str) -> DetectedFormat {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return DetectedFormat::Text;
    }
    if (trimmed.contains("proxies:")
        || trimmed.contains("proxy-groups:")
        || trimmed.contains("rules:"))
        && serde_yaml::from_str::<serde_yaml::Value>(trimmed).is_ok()
    {
        return DetectedFormat::Yaml;
    }
    if trimmed.lines().all(|l| {
        let t = l.trim();
        t.is_empty() || is_likely_base64(t)
    }) {
        let joined = trimmed.split_whitespace().collect::<String>();
        if let Ok(decoded) = decode_base64_url_safe(&joined) {
            if let Ok(s) = String::from_utf8(decoded) {
                if s.contains("://") || s.contains("proxies:") {
                    return DetectedFormat::Base64;
                }
            }
        }
    }
    DetectedFormat::Text
}

fn is_likely_base64(s: &str) -> bool {
    if s.len() % 4 == 1 {
        return false;
    }
    s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '-' || c == '_'
    }) && s.len() >= 8
}

pub fn decode_base64_url_safe(input: &str) -> anyhow::Result<Vec<u8>> {
    let mut s = input.trim().to_owned();
    s.retain(|c| !c.is_whitespace());
    let pad = (4 - s.len() % 4) % 4;
    s.extend(std::iter::repeat_n('=', pad));
    let s_std = s.replace('-', "+").replace('_', "/");
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &s_std)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE, &s))
        .map_err(|e| anyhow!("base64 decode failed: {e}"))
}

pub fn percent_decode(input: &str) -> String {
    percent_decode_str(input).decode_utf8_lossy().into_owned()
}

pub fn parse_subscription_content(input: &str) -> anyhow::Result<Vec<serde_yaml::Value>> {
    match detect_format(input) {
        DetectedFormat::Yaml => {
            let v: serde_yaml::Value = serde_yaml::from_str(input)?;
            if let Some(seq) = v.get("proxies").and_then(|p| p.as_sequence()) {
                Ok(seq.clone())
            } else if let Some(seq) = v.as_sequence() {
                Ok(seq.clone())
            } else {
                Ok(vec![v])
            }
        }
        DetectedFormat::Base64 => {
            let decoded = decode_base64_url_safe(input)?;
            let s = String::from_utf8(decoded)?;
            if s.contains("proxies:") {
                if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&s) {
                    if let Some(seq) = v.get("proxies").and_then(|p| p.as_sequence()) {
                        return Ok(seq.clone());
                    }
                }
            }
            parse_text_links(&s)
        }
        DetectedFormat::Text => parse_text_links(input),
    }
}

pub fn parse_text_links(input: &str) -> anyhow::Result<Vec<serde_yaml::Value>> {
    if input.contains("proxies:") && !input.contains("://") {
        if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(input) {
            if let Some(seq) = v.get("proxies").and_then(|p| p.as_sequence()) {
                return Ok(seq.clone());
            }
        }
    }
    let mut out = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let decoded = if !line.contains("://") && is_likely_base64(line) {
            decode_base64_url_safe(line)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_else(|| line.to_owned())
        } else {
            line.to_owned()
        };
        for part in decoded.split_whitespace() {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            if p.contains("://") {
                match parse_raw_link(p) {
                    Ok(v) => out.push(v),
                    Err(e) => eprintln!("skip link {p}: {e}"),
                }
            }
        }
    }
    Ok(out)
}

pub fn parse_raw_link(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let scheme_end = link.find("://").ok_or_else(|| anyhow!("no scheme"))?;
    let scheme = link[..scheme_end].to_ascii_lowercase();
    match scheme.as_str() {
        "hysteria2" | "hy2" => parse_hysteria2(link),
        "trojan" => parse_trojan(link),
        "vless" => parse_vless(link),
        "vmess" => parse_vmess(link),
        "ss" | "shadowsocks" => parse_ss(link),
        _ => Err(anyhow!("unsupported scheme: {scheme}")),
    }
}

fn parse_url(link: &str) -> anyhow::Result<url::Url> {
    url::Url::parse(link).map_err(|e| anyhow!("url parse: {e}"))
}

fn query_map(url: &url::Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.into_owned(), percent_decode(&v)))
        .collect()
}

fn mapping(pairs: Vec<(&str, serde_yaml::Value)>) -> serde_yaml::Value {
    let mut m = serde_yaml::Mapping::new();
    for (k, v) in pairs {
        m.insert(serde_yaml::Value::String(k.into()), v);
    }
    serde_yaml::Value::Mapping(m)
}

fn parse_hysteria2(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let url = parse_url(link)?;
    let q = query_map(&url);
    let name = percent_decode(url.fragment().unwrap_or("hysteria2"));
    let server = url.host_str().ok_or_else(|| anyhow!("no host"))?.to_owned();
    let port = url.port().unwrap_or(443);
    let password = percent_decode(url.username());
    let mut pairs = vec![
        ("name", serde_yaml::Value::String(name)),
        ("type", serde_yaml::Value::String("hysteria2".into())),
        ("server", serde_yaml::Value::String(server)),
        ("port", serde_yaml::Value::Number(port.into())),
        ("password", serde_yaml::Value::String(password)),
    ];
    if let Some(sni) = q.get("sni").or_else(|| q.get("peer")) {
        pairs.push(("sni", serde_yaml::Value::String(sni.clone())));
    }
    if let Some(insecure) = q.get("insecure").or_else(|| q.get("skip-cert-verify")) {
        let v = insecure == "1" || insecure.eq_ignore_ascii_case("true");
        pairs.push(("skip-cert-verify", serde_yaml::Value::Bool(v)));
    }
    if let Some(obfs) = q.get("obfs") {
        pairs.push(("obfs", serde_yaml::Value::String(obfs.clone())));
    }
    if let Some(obfs_pw) = q.get("obfs-password").or_else(|| q.get("obfs_password")) {
        pairs.push(("obfs-password", serde_yaml::Value::String(obfs_pw.clone())));
    }
    if let Some(up) = q.get("up_mbps").or_else(|| q.get("up")) {
        if let Ok(n) = up.parse::<u64>() {
            pairs.push(("up", serde_yaml::Value::String(format!("{n} Mbps"))));
        }
    }
    if let Some(down) = q.get("down_mbps").or_else(|| q.get("down")) {
        if let Ok(n) = down.parse::<u64>() {
            pairs.push(("down", serde_yaml::Value::String(format!("{n} Mbps"))));
        }
    }
    Ok(mapping(pairs))
}

fn parse_trojan(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let url = parse_url(link)?;
    let q = query_map(&url);
    let name = percent_decode(url.fragment().unwrap_or("trojan"));
    let server = url.host_str().ok_or_else(|| anyhow!("no host"))?.to_owned();
    let port = url.port().unwrap_or(443);
    let password = percent_decode(url.username());
    let mut pairs = vec![
        ("name", serde_yaml::Value::String(name)),
        ("type", serde_yaml::Value::String("trojan".into())),
        ("server", serde_yaml::Value::String(server)),
        ("port", serde_yaml::Value::Number(port.into())),
        ("password", serde_yaml::Value::String(password)),
    ];
    let sni = q
        .get("sni")
        .or_else(|| q.get("peer"))
        .or_else(|| q.get("host"))
        .cloned();
    if let Some(s) = sni {
        pairs.push(("sni", serde_yaml::Value::String(s)));
    }
    if let Some(insecure) = q.get("allowInsecure").or_else(|| q.get("insecure")) {
        let v = insecure == "1" || insecure.eq_ignore_ascii_case("true");
        if v {
            pairs.push(("skip-cert-verify", serde_yaml::Value::Bool(true)));
        }
    }
    if q.get("security").map(|v| v.as_str()) == Some("tls") {
        pairs.push(("skip-cert-verify", serde_yaml::Value::Bool(false)));
    }
    let net = q.get("type").map(|v| v.as_str()).unwrap_or("tcp");
    if net == "ws" {
        pairs.push(("network", serde_yaml::Value::String("ws".into())));
        let mut ws_opts = serde_yaml::Mapping::new();
        if let Some(path) = q.get("path") {
            ws_opts.insert(
                serde_yaml::Value::String("path".into()),
                serde_yaml::Value::String(path.clone()),
            );
        } else if let Some(path) = q.get("serviceName") {
            ws_opts.insert(
                serde_yaml::Value::String("path".into()),
                serde_yaml::Value::String(path.clone()),
            );
        }
        if let Some(host) = q.get("host") {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".into()),
                serde_yaml::Value::String(host.clone()),
            );
            ws_opts.insert(
                serde_yaml::Value::String("headers".into()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        if !ws_opts.is_empty() {
            pairs.push(("ws-opts", serde_yaml::Value::Mapping(ws_opts)));
        }
    } else if net == "grpc" {
        pairs.push(("network", serde_yaml::Value::String("grpc".into())));
        let mut grpc_opts = serde_yaml::Mapping::new();
        if let Some(sn) = q.get("serviceName") {
            grpc_opts.insert(
                serde_yaml::Value::String("grpc-service-name".into()),
                serde_yaml::Value::String(sn.clone()),
            );
        }
        if !grpc_opts.is_empty() {
            pairs.push(("grpc-opts", serde_yaml::Value::Mapping(grpc_opts)));
        }
    }
    if let Some(fp) = q.get("fp") {
        pairs.push(("client-fingerprint", serde_yaml::Value::String(fp.clone())));
    }
    Ok(mapping(pairs))
}

fn parse_vless(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let url = parse_url(link)?;
    let q = query_map(&url);
    let name = percent_decode(url.fragment().unwrap_or("vless"));
    let server = url.host_str().ok_or_else(|| anyhow!("no host"))?.to_owned();
    let port = url.port().unwrap_or(443);
    let uuid = percent_decode(url.username());
    let mut pairs = vec![
        ("name", serde_yaml::Value::String(name)),
        ("type", serde_yaml::Value::String("vless".into())),
        ("server", serde_yaml::Value::String(server)),
        ("port", serde_yaml::Value::Number(port.into())),
        ("uuid", serde_yaml::Value::String(uuid)),
    ];
    if let Some(flow) = q.get("flow") {
        pairs.push(("flow", serde_yaml::Value::String(flow.clone())));
    }
    let security = q.get("security").map(|v| v.as_str()).unwrap_or("none");
    let tls = security == "tls" || security == "reality";
    if tls {
        pairs.push(("tls", serde_yaml::Value::Bool(true)));
    }
    if let Some(sni) = q.get("sni").or_else(|| q.get("serverName")) {
        pairs.push(("servername", serde_yaml::Value::String(sni.clone())));
    }
    if security == "reality" {
        let mut reality_opts = serde_yaml::Mapping::new();
        if let Some(pbk) = q.get("pbk").or_else(|| q.get("publicKey")) {
            reality_opts.insert(
                serde_yaml::Value::String("public-key".into()),
                serde_yaml::Value::String(pbk.clone()),
            );
        }
        if let Some(sid) = q.get("sid").or_else(|| q.get("shortId")) {
            reality_opts.insert(
                serde_yaml::Value::String("short-id".into()),
                serde_yaml::Value::String(sid.clone()),
            );
        }
        if let Some(spx) = q.get("spx") {
            reality_opts.insert(
                serde_yaml::Value::String("spider-x".into()),
                serde_yaml::Value::String(spx.clone()),
            );
        }
        if !reality_opts.is_empty() {
            pairs.push(("reality-opts", serde_yaml::Value::Mapping(reality_opts)));
        }
    }
    if let Some(fp) = q.get("fp") {
        pairs.push(("client-fingerprint", serde_yaml::Value::String(fp.clone())));
    }
    let net = q.get("type").map(|v| v.as_str()).unwrap_or("tcp");
    pairs.push(("network", serde_yaml::Value::String(net.to_owned())));
    if net == "ws" {
        let mut ws_opts = serde_yaml::Mapping::new();
        if let Some(path) = q.get("path") {
            ws_opts.insert(
                serde_yaml::Value::String("path".into()),
                serde_yaml::Value::String(path.clone()),
            );
        }
        if let Some(host) = q.get("host") {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".into()),
                serde_yaml::Value::String(host.clone()),
            );
            ws_opts.insert(
                serde_yaml::Value::String("headers".into()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        if !ws_opts.is_empty() {
            pairs.push(("ws-opts", serde_yaml::Value::Mapping(ws_opts)));
        }
    } else if net == "grpc" {
        let mut grpc_opts = serde_yaml::Mapping::new();
        if let Some(sn) = q.get("serviceName") {
            grpc_opts.insert(
                serde_yaml::Value::String("grpc-service-name".into()),
                serde_yaml::Value::String(sn.clone()),
            );
        }
        if !grpc_opts.is_empty() {
            pairs.push(("grpc-opts", serde_yaml::Value::Mapping(grpc_opts)));
        }
    }
    if let Some(insecure) = q.get("allowInsecure") {
        let v = insecure == "1" || insecure.eq_ignore_ascii_case("true");
        if v {
            pairs.push(("skip-cert-verify", serde_yaml::Value::Bool(true)));
        }
    }
    Ok(mapping(pairs))
}

fn parse_vmess(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let b64 = link["vmess://".len()..]
        .split('#')
        .next()
        .unwrap_or("")
        .trim();
    let name_from_frag = link.split('#').nth(1).map(percent_decode);
    let decoded = decode_base64_url_safe(b64)?;
    let s = String::from_utf8(decoded).context("vmess utf8")?;
    let v: serde_json::Value = serde_json::from_str(&s).context("vmess json")?;
    let server = v
        .get("add")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("vmess no add"))?
        .to_owned();
    let port: u16 = v
        .get("port")
        .and_then(|x| {
            x.as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| x.as_u64().map(|n| n as u16))
        })
        .ok_or_else(|| anyhow!("vmess no port"))?;
    let uuid = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_owned();
    let name = name_from_frag
        .or_else(|| v.get("ps").and_then(|x| x.as_str()).map(|s| s.to_owned()))
        .unwrap_or_else(|| "vmess".to_owned());
    let net = v.get("net").and_then(|x| x.as_str()).unwrap_or("tcp");
    let tls = v.get("tls").and_then(|x| x.as_str()).unwrap_or("none") == "tls";
    let mut pairs = vec![
        ("name", serde_yaml::Value::String(percent_decode(&name))),
        ("type", serde_yaml::Value::String("vmess".into())),
        ("server", serde_yaml::Value::String(server)),
        ("port", serde_yaml::Value::Number(port.into())),
        ("uuid", serde_yaml::Value::String(uuid)),
    ];
    if let Some(aid) = v
        .get("aid")
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse::<u64>().ok())
    {
        pairs.push(("alterId", serde_yaml::Value::Number(aid.into())));
    } else if let Some(aid) = v.get("aid").and_then(|x| x.as_u64()) {
        pairs.push(("alterId", serde_yaml::Value::Number(aid.into())));
    }
    let cipher = v.get("scy").and_then(|x| x.as_str()).unwrap_or("auto");
    pairs.push(("cipher", serde_yaml::Value::String(cipher.to_owned())));
    if tls {
        pairs.push(("tls", serde_yaml::Value::Bool(true)));
        if let Some(sni) = v
            .get("sni")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            pairs.push(("servername", serde_yaml::Value::String(sni.to_owned())));
        } else if let Some(host) = v
            .get("host")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            pairs.push(("servername", serde_yaml::Value::String(host.to_owned())));
        }
        if let Some(fp) = v
            .get("fp")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            pairs.push((
                "client-fingerprint",
                serde_yaml::Value::String(fp.to_owned()),
            ));
        }
    }
    pairs.push(("network", serde_yaml::Value::String(net.to_owned())));
    if net == "ws" {
        let mut ws_opts = serde_yaml::Mapping::new();
        let path = v.get("path").and_then(|x| x.as_str()).unwrap_or("/");
        ws_opts.insert(
            serde_yaml::Value::String("path".into()),
            serde_yaml::Value::String(path.to_owned()),
        );
        if let Some(host) = v
            .get("host")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".into()),
                serde_yaml::Value::String(host.to_owned()),
            );
            ws_opts.insert(
                serde_yaml::Value::String("headers".into()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        pairs.push(("ws-opts", serde_yaml::Value::Mapping(ws_opts)));
    } else if net == "grpc" {
        let mut grpc_opts = serde_yaml::Mapping::new();
        if let Some(sn) = v
            .get("path")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
        {
            grpc_opts.insert(
                serde_yaml::Value::String("grpc-service-name".into()),
                serde_yaml::Value::String(sn.to_owned()),
            );
        }
        if !grpc_opts.is_empty() {
            pairs.push(("grpc-opts", serde_yaml::Value::Mapping(grpc_opts)));
        }
    }
    Ok(mapping(pairs))
}

fn parse_ss(link: &str) -> anyhow::Result<serde_yaml::Value> {
    let after = &link[5..];
    let (main, frag) = after.split_once('#').unwrap_or((after, ""));
    let name = if frag.is_empty() {
        "ss".to_owned()
    } else {
        percent_decode(frag)
    };
    let (method_password, host_port) = if main.contains('@') {
        let at = main.rfind('@').unwrap();
        let left = &main[..at];
        let right = &main[at + 1..];
        let decoded_left = decode_base64_url_safe(left)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| left.to_owned());
        if decoded_left.contains(':') && !left.contains(':') {
            (decoded_left, right.to_owned())
        } else if left.contains(':') {
            let lp = percent_decode(left);
            (lp, right.to_owned())
        } else {
            let combined = format!("{left}@{right}");
            let decoded = decode_base64_url_safe(&combined)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or(combined);
            let at2 = decoded.rfind('@').ok_or_else(|| anyhow!("ss no @"))?;
            (decoded[..at2].to_owned(), decoded[at2 + 1..].to_owned())
        }
    } else {
        let decoded = decode_base64_url_safe(main)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| main.to_owned());
        let at = decoded.rfind('@').ok_or_else(|| anyhow!("ss no @"))?;
        (decoded[..at].to_owned(), decoded[at + 1..].to_owned())
    };
    let (method, password) = method_password
        .split_once(':')
        .ok_or_else(|| anyhow!("ss no method:password"))?;
    let (host, port_str) = host_port
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("ss no host:port"))?;
    let port_str = port_str
        .split('/')
        .next()
        .unwrap_or(port_str)
        .split('?')
        .next()
        .unwrap_or(port_str);
    let port: u16 = port_str.parse().map_err(|_| anyhow!("ss bad port"))?;
    let mut pairs = vec![
        ("name", serde_yaml::Value::String(name)),
        ("type", serde_yaml::Value::String("ss".into())),
        ("server", serde_yaml::Value::String(host.to_owned())),
        ("port", serde_yaml::Value::Number(port.into())),
        ("cipher", serde_yaml::Value::String(method.to_owned())),
        ("password", serde_yaml::Value::String(password.to_owned())),
    ];
    if main.contains("plugin=") {
        if let Ok(url) = url::Url::parse(&format!("ss://{main}")) {
            let q = query_map(&url);
            if let Some(plugin) = q.get("plugin") {
                if plugin.contains("obfs") {
                    pairs.push(("plugin", serde_yaml::Value::String("obfs".into())));
                    let mut opts = serde_yaml::Mapping::new();
                    if plugin.contains("obfs-host=") {
                        if let Some(h) = plugin
                            .split("obfs-host=")
                            .nth(1)
                            .and_then(|s| s.split(';').next())
                        {
                            opts.insert(
                                serde_yaml::Value::String("host".into()),
                                serde_yaml::Value::String(h.to_owned()),
                            );
                        }
                    }
                    if !opts.is_empty() {
                        pairs.push(("plugin-opts", serde_yaml::Value::Mapping(opts)));
                    }
                }
            }
        }
    }
    Ok(mapping(pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_yaml() {
        let yaml = "proxies:\n  - name: test\n    type: ss\n";
        assert_eq!(detect_format(yaml), DetectedFormat::Yaml);
    }

    #[test]
    fn detect_base64() {
        let raw = "trojan://pass@example.com:443#test";
        let b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw.as_bytes());
        assert_eq!(detect_format(&b64), DetectedFormat::Base64);
    }

    #[test]
    fn parse_hysteria2_fm_json() {
        let link = "hysteria2://letmein123@de01.skill-up.store:8443/?sni=de01.skill-up.store&insecure=1#HY2-FM";
        let v = parse_raw_link(link).unwrap();
        let m = v.as_mapping().unwrap();
        assert_eq!(
            m.get(serde_yaml::Value::String("type".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "hysteria2"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("server".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "de01.skill-up.store"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("password".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "letmein123"
        );
    }

    #[test]
    fn parse_trojan_ws() {
        let link = "trojan://p4ssw0rd@de01.skill-up.store:443?security=tls&type=ws&path=%2Ftrojan-ws&host=de01.skill-up.store&sni=de01.skill-up.store#Trojan-WS";
        let v = parse_raw_link(link).unwrap();
        let m = v.as_mapping().unwrap();
        assert_eq!(
            m.get(serde_yaml::Value::String("type".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "trojan"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("network".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "ws"
        );
        let ws = m
            .get(serde_yaml::Value::String("ws-opts".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            ws.get(serde_yaml::Value::String("path".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "/trojan-ws"
        );
    }

    #[test]
    fn parse_vless_reality_pbk() {
        let link = "vless://550e8400-e29b-41d4-a716-446655440000@de01.skill-up.store:443?security=reality&pbk=AbCdEfGhIjKlMnOpQrStUvWxYz1234567890ABC&fp=chrome&sni=de01.skill-up.store&sid=abcd1234&type=tcp&flow=xtls-rprx-vision#VLESS-Reality";
        let v = parse_raw_link(link).unwrap();
        let m = v.as_mapping().unwrap();
        assert_eq!(
            m.get(serde_yaml::Value::String("type".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "vless"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("uuid".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        let ro = m
            .get(serde_yaml::Value::String("reality-opts".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            ro.get(serde_yaml::Value::String("public-key".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "AbCdEfGhIjKlMnOpQrStUvWxYz1234567890ABC"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("flow".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "xtls-rprx-vision"
        );
    }

    #[test]
    fn parse_vmess() {
        let json = serde_json::json!({
            "v": "2",
            "ps": "VMess-WS",
            "add": "de01.skill-up.store",
            "port": "443",
            "id": "b831381d-632b-4d55-b318-8d63f00a1a7c",
            "aid": "0",
            "net": "ws",
            "type": "none",
            "host": "de01.skill-up.store",
            "path": "/vmess-ws",
            "tls": "tls",
            "sni": "de01.skill-up.store"
        });
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            json.to_string().as_bytes(),
        );
        let link = format!("vmess://{b64}");
        let v = parse_raw_link(&link).unwrap();
        let m = v.as_mapping().unwrap();
        assert_eq!(
            m.get(serde_yaml::Value::String("type".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "vmess"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("server".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "de01.skill-up.store"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("network".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "ws"
        );
    }

    #[test]
    fn parse_ss() {
        let link = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQxMjM=@de01.skill-up.store:8388#SS-Test";
        let v = parse_raw_link(link).unwrap();
        let m = v.as_mapping().unwrap();
        assert_eq!(
            m.get(serde_yaml::Value::String("type".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "ss"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("cipher".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "aes-256-gcm"
        );
        assert_eq!(
            m.get(serde_yaml::Value::String("password".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "password123"
        );
    }

    #[test]
    fn parse_text_multiple() {
        let input = "hysteria2://pass@de01.skill-up.store:8443#t1\ntrojan://pass@de01.skill-up.store:443?type=ws&path=%2Fws#t2\n";
        let v = parse_text_links(input).unwrap();
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn dedup_and_base64_detect() {
        let yaml = "proxies:\n  - name: a\n    type: ss\n    server: 1.1.1.1\n    port: 443\n    cipher: aes-256-gcm\n    password: p\n";
        let b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, yaml.as_bytes());
        let out = parse_subscription_content(&b64).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn percent_decode_works() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("test%2Fpath"), "test/path");
    }
}
