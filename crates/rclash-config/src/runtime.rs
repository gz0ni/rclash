use std::path::PathBuf;

use crate::AppConfig;

pub fn runtime_dir() -> Option<PathBuf> {
    crate::config_dir().map(|d| d.join("runtime"))
}

pub fn ensure_runtime_dir() -> anyhow::Result<PathBuf> {
    let dir = runtime_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn runtime_config_path() -> Option<PathBuf> {
    runtime_dir().map(|d| d.join("config.yaml"))
}

fn set_key(mapping: &mut serde_yaml::Mapping, key: &str, value: serde_yaml::Value) {
    mapping.insert(serde_yaml::Value::String(key.into()), value);
}

pub fn assemble_runtime_config(
    profile_content: &str,
    app: &AppConfig,
    secret: &str,
) -> anyhow::Result<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(profile_content)?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("profile root not mapping"))?;
    let mut out = mapping.clone();

    let mixed_port = app.mixed_port.unwrap_or(7890);
    set_key(
        &mut out,
        "mixed-port",
        serde_yaml::Value::Number(mixed_port.into()),
    );
    if let Some(p) = app.socks_port {
        set_key(&mut out, "socks-port", serde_yaml::Value::Number(p.into()));
    }
    let controller = app
        .external_controller
        .clone()
        .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
    set_key(
        &mut out,
        "external-controller",
        serde_yaml::Value::String(controller),
    );
    set_key(
        &mut out,
        "secret",
        serde_yaml::Value::String(secret.to_owned()),
    );
    set_key(
        &mut out,
        "log-level",
        serde_yaml::Value::String(app.log_level.as_str().to_owned()),
    );
    set_key(
        &mut out,
        "mode",
        serde_yaml::Value::String(app.mode.as_str().to_owned()),
    );
    if let Some(v) = app.allow_lan {
        set_key(&mut out, "allow-lan", serde_yaml::Value::Bool(v));
    }
    if let Some(v) = app.ipv6 {
        set_key(&mut out, "ipv6", serde_yaml::Value::Bool(v));
    }
    if let Some(v) = app.unified_delay {
        set_key(&mut out, "unified-delay", serde_yaml::Value::Bool(v));
    }
    if let Some(v) = app.tcp_concurrent {
        set_key(&mut out, "tcp-concurrent", serde_yaml::Value::Bool(v));
    }
    if let Some(v) = app.keep_alive_interval {
        set_key(
            &mut out,
            "keep-alive-interval",
            serde_yaml::Value::Number(v.into()),
        );
    }
    if let Some(ref v) = app.geodata_loader {
        set_key(
            &mut out,
            "geodata-loader",
            serde_yaml::Value::String(v.clone()),
        );
    }
    if let Some(ref v) = app.find_process_mode {
        set_key(
            &mut out,
            "find-process-mode",
            serde_yaml::Value::String(v.clone()),
        );
    }
    for (key, value) in [
        ("geodata-mode", serde_yaml::Value::Bool(true)),
        ("geo-auto-update", serde_yaml::Value::Bool(true)),
        ("geo-update-interval", serde_yaml::Value::Number(24.into())),
    ] {
        if !out.contains_key(serde_yaml::Value::String(key.into())) {
            set_key(&mut out, key, value);
        }
    }
    if !out.contains_key(serde_yaml::Value::String("geox-url".into())) {
        let mut geox = serde_yaml::Mapping::new();
        for (key, url) in [
            (
                "geoip",
                "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat",
            ),
            (
                "geosite",
                "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat",
            ),
            (
                "mmdb",
                "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.metadb",
            ),
        ] {
            geox.insert(
                serde_yaml::Value::String(key.into()),
                serde_yaml::Value::String(url.into()),
            );
        }
        set_key(&mut out, "geox-url", serde_yaml::Value::Mapping(geox));
    }
    if app.tun_enabled {
        let mut tun = match out.get(serde_yaml::Value::String("tun".into())) {
            Some(serde_yaml::Value::Mapping(m)) => m.clone(),
            _ => serde_yaml::Mapping::new(),
        };
        tun.insert(
            serde_yaml::Value::String("enable".into()),
            serde_yaml::Value::Bool(true),
        );
        if !tun.contains_key(serde_yaml::Value::String("stack".into())) {
            tun.insert(
                serde_yaml::Value::String("stack".into()),
                serde_yaml::Value::String("gvisor".into()),
            );
        }
        if !tun.contains_key(serde_yaml::Value::String("auto-route".into())) {
            tun.insert(
                serde_yaml::Value::String("auto-route".into()),
                serde_yaml::Value::Bool(true),
            );
        }
        set_key(&mut out, "tun", serde_yaml::Value::Mapping(tun));
    } else {
        let mut tun = match out.get(serde_yaml::Value::String("tun".into())) {
            Some(serde_yaml::Value::Mapping(m)) => m.clone(),
            _ => serde_yaml::Mapping::new(),
        };
        tun.insert(
            serde_yaml::Value::String("enable".into()),
            serde_yaml::Value::Bool(false),
        );
        set_key(&mut out, "tun", serde_yaml::Value::Mapping(tun));
    }
    if let Ok(dns) = serde_yaml::to_value(app.dns.to_json()) {
        set_key(&mut out, "dns", dns);
    }
    if !app.hosts.is_empty() {
        let mut hosts = serde_yaml::Mapping::new();
        for (k, v) in &app.hosts {
            hosts.insert(
                serde_yaml::Value::String(k.clone()),
                serde_yaml::Value::String(v.clone()),
            );
        }
        set_key(&mut out, "hosts", serde_yaml::Value::Mapping(hosts));
    }

    Ok(serde_yaml::to_string(&serde_yaml::Value::Mapping(out))?)
}

pub fn write_runtime_config(content: &str) -> anyhow::Result<PathBuf> {
    let dir = ensure_runtime_dir()?;
    let path = dir.join("config.yaml");
    crate::atomic_write(&path, content.as_bytes())?;
    Ok(path)
}

pub fn parse_missing_geodata(stderr: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for marker in ["[GEOSITE,", "[GEOIP,"] {
        let typ = marker[1..marker.len() - 1].to_owned();
        let mut search = stderr;
        while let Some(pos) = search.find(marker) {
            let rest = &search[pos + marker.len()..];
            if let Some(end) = rest.find([',', ']']) {
                let name = rest[..end].trim().to_owned();
                if !name.is_empty() && !out.iter().any(|(t, n)| *t == typ && *n == name) {
                    out.push((typ.clone(), name));
                }
            }
            search = &search[pos + marker.len()..];
        }
    }
    out
}

pub fn strip_missing_geodata_rules(
    config_yaml: &str,
    missing: &[(String, String)],
) -> anyhow::Result<(String, usize)> {
    let value: serde_yaml::Value = serde_yaml::from_str(config_yaml)?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("profile root not mapping"))?;
    let mut out = mapping.clone();
    let Some(rules) = out
        .get(serde_yaml::Value::String("rules".into()))
        .and_then(|r| r.as_sequence())
        .cloned()
    else {
        return Ok((config_yaml.to_owned(), 0));
    };
    let mut removed = 0;
    let kept: Vec<serde_yaml::Value> = rules
        .into_iter()
        .filter(|r| {
            let Some(line) = r.as_str() else {
                return true;
            };
            let mut parts = line.split(',');
            let (Some(typ), Some(name)) = (parts.next(), parts.next()) else {
                return true;
            };
            let hit = missing
                .iter()
                .any(|(t, n)| t == typ.trim() && n == name.trim());
            if hit {
                removed += 1;
                return false;
            }
            true
        })
        .collect();
    set_key(&mut out, "rules", serde_yaml::Value::Sequence(kept));
    Ok((
        serde_yaml::to_string(&serde_yaml::Value::Mapping(out))?,
        removed,
    ))
}

fn proxies_seq_mut(mapping: &mut serde_yaml::Mapping) -> &mut Vec<serde_yaml::Value> {
    let entry = mapping
        .entry(serde_yaml::Value::String("proxies".into()))
        .or_insert(serde_yaml::Value::Sequence(vec![]));
    if !entry.is_sequence() {
        *entry = serde_yaml::Value::Sequence(vec![]);
    }
    entry.as_sequence_mut().expect("proxies is sequence")
}

fn proxy_name(v: &serde_yaml::Value) -> Option<&str> {
    v.as_mapping()?
        .get(serde_yaml::Value::String("name".into()))?
        .as_str()
}

pub fn merge_extra_proxies(
    profile_content: &str,
    extras: &[serde_yaml::Value],
) -> anyhow::Result<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(profile_content)?;
    let mapping = value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("profile root not mapping"))?;
    let mut out = mapping.clone();
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let seq = proxies_seq_mut(&mut out);
    for p in seq.iter() {
        if let Some(n) = proxy_name(p) {
            names.insert(n.to_owned());
        }
    }
    let mut added = Vec::new();
    for extra in extras {
        let Some(n) = proxy_name(extra) else {
            continue;
        };
        if names.insert(n.to_owned()) {
            added.push(n.to_owned());
            seq.push(extra.clone());
        }
    }
    if !added.is_empty() {
        let groups_entry = out
            .entry(serde_yaml::Value::String("proxy-groups".into()))
            .or_insert(serde_yaml::Value::Sequence(vec![]));
        if !groups_entry.is_sequence() {
            *groups_entry = serde_yaml::Value::Sequence(vec![]);
        }
        let groups = groups_entry.as_sequence_mut().expect("groups is sequence");
        let mut wired = false;
        for g in groups.iter_mut() {
            let Some(gm) = g.as_mapping_mut() else {
                continue;
            };
            let is_proxy = gm
                .get(serde_yaml::Value::String("name".into()))
                .and_then(|v| v.as_str())
                == Some("PROXY");
            if is_proxy {
                let field = gm
                    .entry(serde_yaml::Value::String("proxies".into()))
                    .or_insert(serde_yaml::Value::Sequence(vec![]));
                if let Some(list) = field.as_sequence_mut() {
                    for n in &added {
                        let val = serde_yaml::Value::String(n.clone());
                        if !list.contains(&val) {
                            list.push(val);
                        }
                    }
                }
                wired = true;
                break;
            }
        }
        if !wired {
            let mut g = serde_yaml::Mapping::new();
            g.insert(
                serde_yaml::Value::String("name".into()),
                serde_yaml::Value::String("PROXY".into()),
            );
            g.insert(
                serde_yaml::Value::String("type".into()),
                serde_yaml::Value::String("select".into()),
            );
            g.insert(
                serde_yaml::Value::String("proxies".into()),
                serde_yaml::Value::Sequence(
                    added.into_iter().map(serde_yaml::Value::String).collect(),
                ),
            );
            groups.push(serde_yaml::Value::Mapping(g));
        }
    }
    Ok(serde_yaml::to_string(&serde_yaml::Value::Mapping(out))?)
}

pub fn build_raw_keys_config(keys: &[serde_yaml::Value]) -> anyhow::Result<String> {
    let mut seen = std::collections::HashSet::new();
    let mut proxies = Vec::new();
    let mut names = Vec::new();
    for k in keys {
        let Some(n) = proxy_name(k) else {
            continue;
        };
        if seen.insert(n.to_owned()) {
            names.push(serde_yaml::Value::String(n.to_owned()));
            proxies.push(k.clone());
        }
    }
    let mut proxy_group = serde_yaml::Mapping::new();
    proxy_group.insert(
        serde_yaml::Value::String("name".into()),
        serde_yaml::Value::String("PROXY".into()),
    );
    proxy_group.insert(
        serde_yaml::Value::String("type".into()),
        serde_yaml::Value::String("select".into()),
    );
    proxy_group.insert(
        serde_yaml::Value::String("proxies".into()),
        serde_yaml::Value::Sequence(names),
    );
    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::String("proxies".into()),
        serde_yaml::Value::Sequence(proxies),
    );
    root.insert(
        serde_yaml::Value::String("proxy-groups".into()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::Mapping(proxy_group)]),
    );
    root.insert(
        serde_yaml::Value::String("rules".into()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("MATCH,PROXY".into())]),
    );
    Ok(serde_yaml::to_string(&serde_yaml::Value::Mapping(root))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogLevel;

    #[test]
    fn assemble_patches_controller_secret_ports() {
        let profile = "proxies: []\nproxy-groups: []\nrules:\n  - MATCH,DIRECT\n";
        let app = AppConfig::default();
        let out = assemble_runtime_config(profile, &app, "s3cret").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        assert_eq!(get("mixed-port").and_then(|v| v.as_u64()), Some(7890));
        assert_eq!(
            get("external-controller").and_then(|v| v.as_str().map(str::to_owned)),
            Some("127.0.0.1:9090".to_owned())
        );
        assert_eq!(
            get("secret").and_then(|v| v.as_str().map(str::to_owned)),
            Some("s3cret".to_owned())
        );
        assert_eq!(
            get("log-level").and_then(|v| v.as_str().map(str::to_owned)),
            Some(LogLevel::Info.as_str().to_owned())
        );
        assert!(get("proxies").is_some());
    }

    #[test]
    fn assemble_respects_app_overrides_and_tun() {
        let profile = "mixed-port: 1111\n";
        let mut app = AppConfig::default();
        app.mixed_port = Some(7891);
        app.tun_enabled = true;
        let out = assemble_runtime_config(profile, &app, "x").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        assert_eq!(get("mixed-port").and_then(|v| v.as_u64()), Some(7891));
        let tun = get("tun").unwrap();
        assert_eq!(
            tun.get(serde_yaml::Value::String("enable".into()))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn assemble_writes_geox_jsdelivr() {
        let app = AppConfig::default();
        let out = assemble_runtime_config("rules:\n  - MATCH,DIRECT\n", &app, "x").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let geox = v.get(serde_yaml::Value::String("geox-url".into())).unwrap();
        assert_eq!(
            geox.get(serde_yaml::Value::String("geosite".into()))
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some(
                "https://cdn.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat"
                    .to_owned()
            )
        );
        let out2 = assemble_runtime_config(
            "geox-url:\n  geosite: https://example.com/x.dat\nrules:\n  - MATCH,DIRECT\n",
            &app,
            "x",
        )
        .unwrap();
        let v2: serde_yaml::Value = serde_yaml::from_str(&out2).unwrap();
        assert_eq!(
            v2.get(serde_yaml::Value::String("geox-url".into()))
                .and_then(|v| v
                    .get(serde_yaml::Value::String("geosite".into()))
                    .and_then(|u| u.as_str().map(str::to_owned))),
            Some("https://example.com/x.dat".to_owned())
        );
    }

    #[test]
    fn assemble_writes_mode_and_disables_stale_tun() {
        let profile = "mode: global\ntun:\n  enable: true\n  stack: gvisor\n";
        let mut app = AppConfig::default();
        app.mode = crate::CoreMode::Direct;
        app.tun_enabled = false;
        let out = assemble_runtime_config(profile, &app, "x").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        assert_eq!(
            get("mode").and_then(|v| v.as_str().map(str::to_owned)),
            Some("direct".to_owned())
        );
        let tun = get("tun").unwrap();
        assert_eq!(
            tun.get(serde_yaml::Value::String("enable".into()))
                .and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            tun.get(serde_yaml::Value::String("stack".into()))
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some("gvisor".to_owned())
        );
    }

    #[test]
    fn parse_missing_geodata_from_core_stderr() {
        let err = "rules[2] [GEOSITE,adobe-ads,REJECT] error: load GeoSite data error, failed to decode geodata file: GeoSite.dat, base error: list adobe-ads not found in GeoSite.dat";
        assert_eq!(
            parse_missing_geodata(err),
            vec![("GEOSITE".to_owned(), "adobe-ads".to_owned())]
        );
        let wrapped = "time=\"2026-09-04\" level=error msg=\"rules[0] [GEOIP,ru,DIRECT] error: country code ru not found in GeoIP.dat\"";
        assert_eq!(
            parse_missing_geodata(wrapped),
            vec![("GEOIP".to_owned(), "ru".to_owned())]
        );
        assert!(parse_missing_geodata("proxies: [] fatal").is_empty());
    }

    #[test]
    fn strip_missing_geodata_rules_drops_exact_lists() {
        let cfg = "rules:\n  - GEOSITE,adobe-ads,REJECT\n  - GEOSITE,adobe,REJECT\n  - GEOIP,ru,DIRECT,no-resolve\n  - MATCH,PROXY\n";
        let missing = vec![("GEOSITE".to_owned(), "adobe-ads".to_owned())];
        let (out, removed) = strip_missing_geodata_rules(cfg, &missing).unwrap();
        assert_eq!(removed, 1);
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let rules = v
            .get(serde_yaml::Value::String("rules".into()))
            .and_then(|r| r.as_sequence())
            .cloned()
            .unwrap();
        assert_eq!(rules.len(), 3);
        assert!(!rules
            .iter()
            .any(|r| r.as_str().unwrap_or_default().contains("adobe-ads")));
    }

    #[test]
    fn assemble_writes_dns_and_hosts() {
        let profile = "rules:\n  - MATCH,DIRECT\n";
        let mut app = AppConfig::default();
        app.dns.listen = "127.0.0.1:5353".to_owned();
        app.dns.nameservers = vec!["1.1.1.1".to_owned()];
        app.hosts
            .insert("example.com".to_owned(), "1.2.3.4".to_owned());
        let out = assemble_runtime_config(profile, &app, "x").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        let dns = get("dns").unwrap();
        assert_eq!(
            dns.get(serde_yaml::Value::String("listen".into()))
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some("127.0.0.1:5353".to_owned())
        );
        assert_eq!(
            dns.get(serde_yaml::Value::String("enhanced-mode".into()))
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some("fake-ip".to_owned())
        );
        let hosts = get("hosts").unwrap();
        assert_eq!(
            hosts
                .get(serde_yaml::Value::String("example.com".into()))
                .and_then(|v| v.as_str().map(str::to_owned)),
            Some("1.2.3.4".to_owned())
        );
    }

    #[test]
    fn assemble_rejects_non_mapping() {
        let app = AppConfig::default();
        assert!(assemble_runtime_config("- just\n- a\n- list\n", &app, "x").is_err());
    }

    #[test]
    fn assemble_enables_geodata_by_default() {
        let app = AppConfig::default();
        let out = assemble_runtime_config("rules:\n  - MATCH,DIRECT\n", &app, "x").unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        assert_eq!(get("geodata-mode").and_then(|v| v.as_bool()), Some(true));
        let out2 =
            assemble_runtime_config("geodata-mode: false\nrules:\n  - MATCH,DIRECT\n", &app, "x")
                .unwrap();
        let v2: serde_yaml::Value = serde_yaml::from_str(&out2).unwrap();
        assert_eq!(
            v2.get(serde_yaml::Value::String("geodata-mode".into()))
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    fn proxy_val(name: &str) -> serde_yaml::Value {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("name".into()),
            serde_yaml::Value::String(name.into()),
        );
        m.insert(
            serde_yaml::Value::String("type".into()),
            serde_yaml::Value::String("ss".into()),
        );
        serde_yaml::Value::Mapping(m)
    }

    #[test]
    fn merge_appends_and_wires_proxy_group() {
        let profile =
            "proxies:\n  - name: a\n    type: ss\nproxy-groups: []\nrules:\n  - MATCH,DIRECT\n";
        let out = merge_extra_proxies(profile, &[proxy_val("a"), proxy_val("b")]).unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        let proxies = get("proxies")
            .and_then(|v| v.as_sequence().cloned())
            .unwrap();
        assert_eq!(proxies.len(), 2);
        let groups = get("proxy-groups")
            .and_then(|v| v.as_sequence().cloned())
            .unwrap();
        assert_eq!(groups.len(), 1);
        let wired = groups[0]
            .get(serde_yaml::Value::String("proxies".into()))
            .and_then(|v| v.as_sequence().cloned())
            .unwrap();
        assert_eq!(wired.len(), 1);
    }

    #[test]
    fn build_raw_keys_config_minimal() {
        let out = build_raw_keys_config(&[proxy_val("a"), proxy_val("a")]).unwrap();
        let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
        let get = |k: &str| v.get(serde_yaml::Value::String(k.into())).cloned();
        let proxies = get("proxies")
            .and_then(|v| v.as_sequence().cloned())
            .unwrap();
        assert_eq!(proxies.len(), 1);
        let rules = get("rules").and_then(|v| v.as_sequence().cloned()).unwrap();
        assert_eq!(rules.len(), 1);
    }
}
