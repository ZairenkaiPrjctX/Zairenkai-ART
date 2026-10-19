//! The ART module as apexd reports it in `/apex/apex-info-list.xml`.
//!
//! Path: `kApexInfoList` in system/apex `apexd/apex_constants.h` (android-17.0.0_r1).
//! Attributes: `ApexInfoList.xsd` in system/apex at the same tag. Every ART APEX flavour
//! (`com.android.art`, `.debug`, `.testing`, Google-signed builds) uses the manifest name
//! `com.android.art` (art `build/apex/manifest-art*.json`), so the file path tells them apart.
//!
//! The parser handles exactly the shape apexd writes: `<apex-info .../>` elements with quoted
//! attributes. It is not a general XML parser and fails rather than guess on anything else.

use serde::Serialize;

use crate::obs::Obs;
use crate::sysroot::SysRoot;

pub const APEX_INFO_LIST: &str = "/apex/apex-info-list.xml";
pub const ART_MODULE: &str = "com.android.art";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApexInfo {
    pub module_name: String,
    pub module_path: Option<String>,
    pub preinstalled_module_path: Option<String>,
    pub version_code: Option<i64>,
    pub version_name: Option<String>,
    pub is_factory: Option<bool>,
    pub is_active: Option<bool>,
    pub last_update_millis: Option<i64>,
    pub partition: Option<String>,
}

fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail
            .find(';')
            .ok_or_else(|| format!("unterminated entity in {s:?}"))?;
        out.push_str(match &tail[..=end] {
            "&amp;" => "&",
            "&lt;" => "<",
            "&gt;" => ">",
            "&quot;" => "\"",
            "&apos;" => "'",
            other => return Err(format!("unsupported entity {other}")),
        });
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Attributes of one start tag body (the text between the element name and `>`).
fn attributes(body: &str) -> Result<Vec<(String, String)>, String> {
    let mut attrs = Vec::new();
    let mut rest = body.trim_start();
    while !rest.is_empty() && rest != "/" {
        let eq = rest
            .find('=')
            .ok_or_else(|| format!("attribute without value near {rest:?}"))?;
        let name = rest[..eq].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!("bad attribute name {name:?}"));
        }
        let after = rest[eq + 1..].trim_start();
        let quote = after
            .chars()
            .next()
            .filter(|c| *c == '"' || *c == '\'')
            .ok_or_else(|| format!("unquoted value for {name}"))?;
        let close = after[1..]
            .find(quote)
            .ok_or_else(|| format!("unterminated value for {name}"))?;
        attrs.push((name.to_string(), unescape(&after[1..1 + close])?));
        rest = after[close + 2..].trim_start();
    }
    Ok(attrs)
}

fn parse_bool(name: &str, v: &str) -> Result<bool, String> {
    match v {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{name}: not a boolean: {v:?}")),
    }
}

fn parse_i64(name: &str, v: &str) -> Result<i64, String> {
    v.parse()
        .map_err(|_| format!("{name}: not an integer: {v:?}"))
}

/// All `<apex-info>` elements in an apex-info-list document.
pub fn parse(xml: &str) -> Result<Vec<ApexInfo>, String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<apex-info") {
        let tail = &rest[i + "<apex-info".len()..];
        if tail.starts_with("-list") {
            rest = tail;
            continue;
        }
        if !tail.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            return Err("unexpected element name after <apex-info".into());
        }
        // Find the closing '>' outside quotes.
        let mut quote = None;
        let mut end = None;
        for (j, c) in tail.char_indices() {
            match (quote, c) {
                (None, '"' | '\'') => quote = Some(c),
                (Some(q), c) if c == q => quote = None,
                (None, '>') => {
                    end = Some(j);
                    break;
                }
                _ => {}
            }
        }
        let end = end.ok_or("unterminated <apex-info> element")?;
        let mut info = ApexInfo {
            module_name: String::new(),
            module_path: None,
            preinstalled_module_path: None,
            version_code: None,
            version_name: None,
            is_factory: None,
            is_active: None,
            last_update_millis: None,
            partition: None,
        };
        for (name, v) in attributes(&tail[..end])? {
            match name.as_str() {
                "moduleName" => info.module_name = v,
                "modulePath" => info.module_path = Some(v),
                "preinstalledModulePath" => info.preinstalled_module_path = Some(v),
                "versionCode" => info.version_code = Some(parse_i64(&name, &v)?),
                "versionName" => info.version_name = Some(v),
                "isFactory" => info.is_factory = Some(parse_bool(&name, &v)?),
                "isActive" => info.is_active = Some(parse_bool(&name, &v)?),
                "lastUpdateMillis" => info.last_update_millis = Some(parse_i64(&name, &v)?),
                "partition" => info.partition = Some(v),
                _ => {} // e.g. provideSharedApexLibs; not needed here
            }
        }
        if info.module_name.is_empty() {
            return Err("<apex-info> without moduleName".into());
        }
        out.push(info);
        rest = &tail[end..];
    }
    Ok(out)
}

/// ART module entries (active and inactive), as reported by apexd.
pub fn art_module(root: &SysRoot) -> Obs<Vec<ApexInfo>> {
    let text = match root.read_string(APEX_INFO_LIST) {
        Ok(t) => t,
        Err(e) => return Obs::from_io(APEX_INFO_LIST, &e),
    };
    match parse(&text) {
        Ok(all) => {
            let art: Vec<_> = all
                .into_iter()
                .filter(|a| a.module_name == ART_MODULE)
                .collect();
            if art.is_empty() {
                Obs::unavailable(APEX_INFO_LIST, format!("no {ART_MODULE} entry"))
            } else {
                Obs::value(art, APEX_INFO_LIST)
            }
        }
        Err(e) => Obs::unavailable(APEX_INFO_LIST, format!("parse error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<apex-info-list>
    <apex-info moduleName="com.android.art" modulePath="/data/apex/active/com.android.art@370000000.apex" preinstalledModulePath="/system/apex/com.google.android.art.apex" versionCode="370000000" versionName="" isFactory="false" isActive="true" lastUpdateMillis="1760000000000" provideSharedApexLibs="false" partition="SYSTEM"/>
    <apex-info moduleName="com.android.art" modulePath="/system/apex/com.google.android.art.apex" preinstalledModulePath="/system/apex/com.google.android.art.apex" versionCode="361000000" versionName="" isFactory="true" isActive="false" lastUpdateMillis="0" provideSharedApexLibs="false" partition="SYSTEM"/>
    <apex-info moduleName="com.android.tzdata" modulePath="/system/apex/com.android.tzdata.apex" versionCode="1" isFactory="true" isActive="true" />
</apex-info-list>
"#;

    #[test]
    fn parses_apexd_output() {
        let all = parse(LIST).unwrap();
        assert_eq!(all.len(), 3);
        let a = &all[0];
        assert_eq!(a.module_name, "com.android.art");
        assert_eq!(a.version_code, Some(370000000));
        assert_eq!(a.is_active, Some(true));
        assert_eq!(a.is_factory, Some(false));
        assert_eq!(a.partition.as_deref(), Some("SYSTEM"));
        assert_eq!(a.version_name.as_deref(), Some(""));
        assert_eq!(all[2].preinstalled_module_path, None);
    }

    #[test]
    fn entities_and_single_quotes() {
        let x = "<apex-info-list><apex-info moduleName='a&amp;b' versionName=\"x&quot;y\"/></apex-info-list>";
        let all = parse(x).unwrap();
        assert_eq!(all[0].module_name, "a&b");
        assert_eq!(all[0].version_name.as_deref(), Some("x\"y"));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "<apex-info moduleName=\"a\"",
            "<apex-info moduleName=a/>",
            "<apex-info versionCode=\"x\" moduleName=\"a\"/>",
            "<apex-info isActive=\"yes\" moduleName=\"a\"/>",
            "<apex-info modulePath=\"/x\"/>",
            "<apex-info moduleName=\"&bogus;\"/>",
            "<apex-infox moduleName=\"a\"/>",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn art_module_from_fixture_root() {
        let dir = std::env::temp_dir().join(format!("zart-apex-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("apex")).unwrap();
        let root = SysRoot::at(&dir);
        assert!(matches!(art_module(&root), Obs::Unavailable { .. }));
        std::fs::write(dir.join("apex/apex-info-list.xml"), LIST).unwrap();
        let art = art_module(&root);
        assert_eq!(art.get().unwrap().len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
