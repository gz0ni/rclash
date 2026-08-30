use std::collections::HashSet;
use std::path::PathBuf;

pub fn custom_yaml_path() -> Option<PathBuf> {
    crate::profile::profiles_dir().map(|d| d.join("custom.yaml"))
}

pub fn ensure_custom_yaml() -> anyhow::Result<PathBuf> {
    let dir = crate::profile::ensure_profiles_dir()?;
    let path = dir.join("custom.yaml");
    if !path.exists() {
        let initial = serde_yaml::Value::Mapping({
            let mut m = serde_yaml::Mapping::new();
            m.insert(
                serde_yaml::Value::String("proxies".into()),
                serde_yaml::Value::Sequence(vec![]),
            );
            m.insert(
                serde_yaml::Value::String("proxy-groups".into()),
                serde_yaml::Value::Sequence(vec![{
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
                        serde_yaml::Value::Sequence(vec![]),
                    );
                    serde_yaml::Value::Mapping(g)
                }]),
            );
            m.insert(
                serde_yaml::Value::String("rules".into()),
                serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("MATCH,PROXY".into())]),
            );
            m
        });
        let s = serde_yaml::to_string(&initial)?;
        crate::atomic_write(&path, s.as_bytes())?;
    }
    Ok(path)
}

pub fn read_custom_yaml() -> anyhow::Result<serde_yaml::Value> {
    let path = ensure_custom_yaml()?;
    let s = std::fs::read_to_string(&path)?;
    let v: serde_yaml::Value = serde_yaml::from_str(&s)?;
    Ok(v)
}

pub fn write_custom_yaml(value: &serde_yaml::Value) -> anyhow::Result<()> {
    let path = ensure_custom_yaml()?;
    let s = serde_yaml::to_string(value)?;
    crate::atomic_write(&path, s.as_bytes())?;
    Ok(())
}

pub fn list_raw_keys() -> anyhow::Result<Vec<String>> {
    let v = read_custom_yaml()?;
    let seq = v
        .get("proxies")
        .and_then(|p| p.as_sequence())
        .cloned()
        .unwrap_or_default();
    let mut keys = Vec::new();
    for item in seq {
        if let Some(m) = item.as_mapping() {
            if let Some(name) = m.get(serde_yaml::Value::String("name".into())) {
                if let Some(s) = name.as_str() {
                    keys.push(s.to_owned());
                }
            }
        }
    }
    Ok(keys)
}

fn dedup_key(proxy: &serde_yaml::Mapping) -> String {
    let name = proxy
        .get(serde_yaml::Value::String("name".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let server = proxy
        .get(serde_yaml::Value::String("server".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let port = proxy
        .get(serde_yaml::Value::String("port".into()))
        .map(|v| {
            if let Some(n) = v.as_u64() {
                n.to_string()
            } else if let Some(s) = v.as_str() {
                s.to_owned()
            } else {
                String::new()
            }
        })
        .unwrap_or_default();
    let tp = proxy
        .get(serde_yaml::Value::String("type".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !server.is_empty() && !port.is_empty() && !tp.is_empty() {
        format!("{server}:{port}/{tp}")
    } else {
        name.to_owned()
    }
}

pub fn add_raw_proxies(new_proxies: Vec<serde_yaml::Value>) -> anyhow::Result<usize> {
    let mut root = read_custom_yaml()?;
    let mapping = root
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("custom.yaml root not mapping"))?;

    let proxies_entry = mapping
        .entry(serde_yaml::Value::String("proxies".into()))
        .or_insert(serde_yaml::Value::Sequence(vec![]));
    let proxies_seq = proxies_entry
        .as_sequence_mut()
        .ok_or_else(|| anyhow::anyhow!("proxies not sequence"))?;

    let mut existing_keys: HashSet<String> = HashSet::new();
    let mut existing_dedup: HashSet<String> = HashSet::new();
    for p in proxies_seq.iter() {
        if let Some(m) = p.as_mapping() {
            if let Some(name) = m
                .get(serde_yaml::Value::String("name".into()))
                .and_then(|v| v.as_str())
            {
                existing_keys.insert(name.to_owned());
            }
            existing_dedup.insert(dedup_key(m));
        }
    }

    let mut added = 0usize;
    let mut added_names = Vec::new();
    for proxy in new_proxies {
        let Some(m) = proxy.as_mapping() else {
            continue;
        };
        let name = m
            .get(serde_yaml::Value::String("name".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        if name.is_empty() {
            continue;
        }
        if existing_keys.contains(&name) {
            continue;
        }
        let dk = dedup_key(m);
        if existing_dedup.contains(&dk) {
            continue;
        }
        existing_keys.insert(name.clone());
        existing_dedup.insert(dk);
        added_names.push(name);
        proxies_seq.push(proxy);
        added += 1;
    }

    if added > 0 {
        let groups_entry = mapping
            .entry(serde_yaml::Value::String("proxy-groups".into()))
            .or_insert(serde_yaml::Value::Sequence(vec![]));
        if let Some(groups) = groups_entry.as_sequence_mut() {
            let mut found = false;
            for g in groups.iter_mut() {
                if let Some(gm) = g.as_mapping_mut() {
                    let is_proxy = gm
                        .get(serde_yaml::Value::String("name".into()))
                        .and_then(|v| v.as_str())
                        == Some("PROXY");
                    if is_proxy {
                        let proxies_field = gm
                            .entry(serde_yaml::Value::String("proxies".into()))
                            .or_insert(serde_yaml::Value::Sequence(vec![]));
                        if let Some(seq) = proxies_field.as_sequence_mut() {
                            for n in &added_names {
                                let val = serde_yaml::Value::String(n.clone());
                                if !seq.contains(&val) {
                                    seq.push(val);
                                }
                            }
                        }
                        found = true;
                        break;
                    }
                }
            }
            if !found {
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
                        added_names
                            .into_iter()
                            .map(serde_yaml::Value::String)
                            .collect(),
                    ),
                );
                groups.push(serde_yaml::Value::Mapping(g));
            }
        }
        let s = serde_yaml::to_string(&root)?;
        let path = ensure_custom_yaml()?;
        crate::atomic_write(&path, s.as_bytes())?;
    }

    Ok(added)
}

pub fn remove_raw_proxy(name: &str) -> anyhow::Result<bool> {
    let mut root = read_custom_yaml()?;
    let mapping = root
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("custom.yaml root not mapping"))?;

    let mut removed = false;
    if let Some(proxies) = mapping
        .get_mut(serde_yaml::Value::String("proxies".into()))
        .and_then(|v| v.as_sequence_mut())
    {
        let before = proxies.len();
        proxies.retain(|p| {
            p.as_mapping()
                .and_then(|m| m.get(serde_yaml::Value::String("name".into())))
                .and_then(|v| v.as_str())
                != Some(name)
        });
        removed = proxies.len() != before;
    }

    if removed {
        if let Some(groups) = mapping
            .get_mut(serde_yaml::Value::String("proxy-groups".into()))
            .and_then(|v| v.as_sequence_mut())
        {
            for g in groups.iter_mut() {
                if let Some(gm) = g.as_mapping_mut() {
                    let is_proxy = gm
                        .get(serde_yaml::Value::String("name".into()))
                        .and_then(|v| v.as_str())
                        == Some("PROXY");
                    if is_proxy {
                        if let Some(seq) = gm
                            .get_mut(serde_yaml::Value::String("proxies".into()))
                            .and_then(|v| v.as_sequence_mut())
                        {
                            seq.retain(|v| v.as_str() != Some(name));
                        }
                        break;
                    }
                }
            }
        }
        let s = serde_yaml::to_string(&root)?;
        let path = ensure_custom_yaml()?;
        crate::atomic_write(&path, s.as_bytes())?;
    }

    Ok(removed)
}

pub fn clear_raw_proxies() -> anyhow::Result<()> {
    let mut root = read_custom_yaml()?;
    if let Some(m) = root.as_mapping_mut() {
        m.insert(
            serde_yaml::Value::String("proxies".into()),
            serde_yaml::Value::Sequence(vec![]),
        );
        if let Some(groups) = m
            .get_mut(serde_yaml::Value::String("proxy-groups".into()))
            .and_then(|v| v.as_sequence_mut())
        {
            for g in groups.iter_mut() {
                if let Some(gm) = g.as_mapping_mut() {
                    let is_proxy = gm
                        .get(serde_yaml::Value::String("name".into()))
                        .and_then(|v| v.as_str())
                        == Some("PROXY");
                    if is_proxy {
                        gm.insert(
                            serde_yaml::Value::String("proxies".into()),
                            serde_yaml::Value::Sequence(vec![]),
                        );
                        break;
                    }
                }
            }
        }
        let s = serde_yaml::to_string(&root)?;
        let path = ensure_custom_yaml()?;
        crate::atomic_write(&path, s.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping_from_pairs(pairs: Vec<(&str, &str)>) -> serde_yaml::Value {
        let mut m = serde_yaml::Mapping::new();
        for (k, v) in pairs {
            if k == "port" {
                if let Ok(n) = v.parse::<u64>() {
                    m.insert(
                        serde_yaml::Value::String(k.into()),
                        serde_yaml::Value::Number(serde_yaml::Number::from(n)),
                    );
                } else {
                    m.insert(
                        serde_yaml::Value::String(k.into()),
                        serde_yaml::Value::String(v.into()),
                    );
                }
            } else {
                m.insert(
                    serde_yaml::Value::String(k.into()),
                    serde_yaml::Value::String(v.into()),
                );
            }
        }
        serde_yaml::Value::Mapping(m)
    }

    #[test]
    fn dedup_key_server_port_type() {
        let v = mapping_from_pairs(vec![
            ("name", "MyProxy"),
            ("server", "1.2.3.4"),
            ("port", "443"),
            ("type", "trojan"),
        ]);
        let m = v.as_mapping().unwrap();
        assert_eq!(dedup_key(m), "1.2.3.4:443/trojan");
    }

    #[test]
    fn dedup_key_fallback_name() {
        let v = mapping_from_pairs(vec![("name", "MyProxy")]);
        let m = v.as_mapping().unwrap();
        assert_eq!(dedup_key(m), "MyProxy");
    }
}
