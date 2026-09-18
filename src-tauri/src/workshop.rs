//! Steam Workshop metadata: titles, descriptions, update times and required items for the
//! installed items, cached in `workshop.cache.json`.
//!
//! Sources, in order: (1) the keyless Web API `ISteamRemoteStorage/GetPublishedFileDetails/v1`
//! for the basic fields; (2) the item's public page for the "Required items" list (the keyless
//! API does not return children); (3) the CA launcher's own cache (`20190104-moddata.dat`) as
//! an offline seed for titles.

use crate::json_store;
use crate::paths;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

const USER_AGENT: &str = "TKModManager";
const DETAILS_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkshopItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub time_updated: u64,
    pub time_created: u64,
    pub tags: Vec<String>,
    pub preview_url: String,
    pub required_items: Vec<String>,
    pub fetched_at: u64,
    pub from_launcher_cache: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Cache {
    #[serde(default)]
    pub items: HashMap<String, WorkshopItem>,
}

fn cache_path() -> PathBuf {
    paths::app_data_dir().join("workshop.cache.json")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn load_cache() -> Cache {
    json_store::load::<Cache>(&cache_path()).unwrap_or_default()
}

/// Everything cached, plus launcher-cache seeds for ids that have nothing yet.
#[tauri::command]
pub fn workshop_cached() -> HashMap<String, WorkshopItem> {
    let mut cache = load_cache();
    let mut changed = false;
    for (id, item) in launcher_cache_items() {
        if !cache.items.contains_key(&id) {
            cache.items.insert(id, item);
            changed = true;
        }
    }
    if changed {
        let _ = json_store::save(&cache_path(), &cache);
    }
    cache.items
}

/// Fetch details for `ids` (skipping fresh cache entries unless `force`). Errors are per-item
/// tolerant: whatever Steam returned is cached and returned; a network failure is an error.
#[tauri::command]
pub async fn workshop_fetch(state: State<'_, AppState>, ids: Vec<String>, force: bool) -> Result<HashMap<String, WorkshopItem>, String> {
    let ttl_secs = u64::from(state.settings().workshop_cache_hours.max(1)) * 3600;
    let mut cache = load_cache();
    let t = now();
    let stale: Vec<String> = ids
        .iter()
        .filter(|id| {
            force
                || cache
                    .items
                    .get(*id)
                    .map(|it| it.from_launcher_cache || t.saturating_sub(it.fetched_at) > ttl_secs)
                    .unwrap_or(true)
        })
        .cloned()
        .collect();
    if stale.is_empty() {
        return Ok(cache.items.into_iter().filter(|(k, _)| ids.contains(k)).collect());
    }
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;
    for chunk in stale.chunks(50) {
        let details = fetch_details(&client, chunk).await?;
        for mut item in details {
            item.fetched_at = t;
            item.required_items = fetch_required_items(&client, &item.id).await.unwrap_or_default();
            cache.items.insert(item.id.clone(), item);
        }
    }
    json_store::save(&cache_path(), &cache)?;
    Ok(cache.items.into_iter().filter(|(k, _)| ids.contains(k)).collect())
}

async fn fetch_details(client: &reqwest::Client, ids: &[String]) -> Result<Vec<WorkshopItem>, String> {
    let mut form: Vec<(String, String)> = vec![("itemcount".into(), ids.len().to_string())];
    for (i, id) in ids.iter().enumerate() {
        form.push((format!("publishedfileids[{i}]"), id.clone()));
    }
    let resp = client
        .post(DETAILS_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Steam API: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Steam API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Steam API JSON: {e}"))?;
    Ok(parse_details(&json))
}

pub fn parse_details(json: &serde_json::Value) -> Vec<WorkshopItem> {
    let Some(arr) = json["response"]["publishedfiledetails"].as_array() else { return vec![] };
    arr.iter()
        .filter(|d| d["result"].as_u64() == Some(1))
        .map(|d| WorkshopItem {
            id: d["publishedfileid"].as_str().unwrap_or("").to_string(),
            title: d["title"].as_str().unwrap_or("").to_string(),
            description: d["description"].as_str().unwrap_or("").to_string(),
            time_updated: d["time_updated"].as_u64().unwrap_or(0),
            time_created: d["time_created"].as_u64().unwrap_or(0),
            tags: d["tags"]
                .as_array()
                .map(|t| t.iter().filter_map(|x| x["tag"].as_str().map(str::to_owned)).collect())
                .unwrap_or_default(),
            preview_url: d["preview_url"].as_str().unwrap_or("").to_string(),
            required_items: vec![],
            fetched_at: 0,
            from_launcher_cache: false,
        })
        .filter(|i| !i.id.is_empty())
        .collect()
}

/// Scrape the "Required items" box of the public Workshop page. Best effort.
async fn fetch_required_items(client: &reqwest::Client, id: &str) -> Result<Vec<String>, String> {
    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}");
    let html = client.get(&url).send().await.map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    Ok(parse_required_items(&html, id))
}

pub fn parse_required_items(html: &str, self_id: &str) -> Vec<String> {
    let Some(start) = html.find("requiredItemsContainer") else { return vec![] };
    let rest = &html[start..];
    let end = rest.find("</div>").map(|e| e + 6).unwrap_or(rest.len());
    // The container holds one <a href="...?id=NNN"> per required item; collect until the
    // section closes. Nested divs are rare here; be generous and scan a bounded window.
    let window = &rest[..end.max(4000.min(rest.len()))];
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(i) = window[pos..].find("filedetails/?id=") {
        let s = pos + i + "filedetails/?id=".len();
        let digits: String = window[s..].chars().take_while(|c| c.is_ascii_digit()).collect();
        pos = s;
        if !digits.is_empty() && digits != self_id && !out.contains(&digits) {
            out.push(digits);
        }
    }
    out
}

/// Titles/descriptions the CA launcher already cached, keyed by Workshop id. Only entries whose
/// `packfile` is inside a Workshop item folder can be mapped to an id.
fn launcher_cache_items() -> Vec<(String, WorkshopItem)> {
    let Some(base) = directories::BaseDirs::new() else { return vec![] };
    let file = base.data_dir().join("The Creative Assembly").join("Launcher").join("20190104-moddata.dat");
    let Ok(bytes) = std::fs::read(&file) else { return vec![] };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return vec![] };
    let Some(arr) = json.as_array() else { return vec![] };
    let needle = format!("/content/{}/", paths::STEAM_APP_ID);
    arr.iter()
        .filter_map(|e| {
            let packfile = e["packfile"].as_str()?.replace('\\', "/");
            let idx = packfile.find(&needle)?;
            let id: String = packfile[idx + needle.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if id.is_empty() {
                return None;
            }
            Some((
                id.clone(),
                WorkshopItem {
                    id,
                    title: e["name"].as_str().unwrap_or("").to_string(),
                    description: e["short"].as_str().unwrap_or("").to_string(),
                    tags: e["category"].as_str().filter(|c| !c.is_empty()).map(|c| vec![c.to_string()]).unwrap_or_default(),
                    from_launcher_cache: true,
                    ..Default::default()
                },
            ))
        })
        .collect()
}

const COLLECTION_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetCollectionDetails/v1/";

/// A Workshop collection id from a URL (`...?id=123`) or a bare number.
pub fn parse_collection_id(input: &str) -> Option<String> {
    let t = input.trim();
    if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        return Some(t.to_string());
    }
    let idx = t.find("id=")? + 3;
    let digits: String = t[idx..].chars().take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

pub fn parse_collection_children(json: &serde_json::Value) -> Vec<String> {
    let Some(children) = json["response"]["collectiondetails"][0]["children"].as_array() else { return vec![] };
    let mut rows: Vec<(u64, String)> = children
        .iter()
        .filter_map(|c| Some((c["sortorder"].as_u64().unwrap_or(0), c["publishedfileid"].as_str()?.to_string())))
        .collect();
    rows.sort_by_key(|r| r.0);
    rows.into_iter().map(|r| r.1).collect()
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CollectionResult {
    pub id: String,
    pub title: String,
    /// Item ids in the collection's own order.
    pub children: Vec<String>,
    /// Details for the children (titles for the ones that are not installed).
    pub items: HashMap<String, WorkshopItem>,
}

/// Resolve a Workshop collection (keyless Web API) into its items.
#[tauri::command]
pub async fn workshop_collection(input: String) -> Result<CollectionResult, String> {
    let id = parse_collection_id(&input).ok_or("that does not look like a Workshop collection link or id")?;
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build().map_err(|e| e.to_string())?;
    let form = [("collectioncount".to_string(), "1".to_string()), ("publishedfileids[0]".to_string(), id.clone())];
    let resp = client.post(COLLECTION_URL).form(&form).send().await.map_err(|e| format!("Steam API: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Steam API returned {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Steam API JSON: {e}"))?;
    let children = parse_collection_children(&json);
    if children.is_empty() {
        return Err("the collection is empty, private, or not a collection".into());
    }
    let title = fetch_details(&client, std::slice::from_ref(&id)).await.ok().and_then(|v| v.into_iter().next()).map(|i| i.title).unwrap_or_default();
    let mut items = HashMap::new();
    for chunk in children.chunks(50) {
        for item in fetch_details(&client, chunk).await? {
            items.insert(item.id.clone(), item);
        }
    }
    Ok(CollectionResult { id, title, children, items })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_ids_and_children() {
        assert_eq!(parse_collection_id("https://steamcommunity.com/sharedfiles/filedetails/?id=2875547086&x=1").as_deref(), Some("2875547086"));
        assert_eq!(parse_collection_id(" 12345 ").as_deref(), Some("12345"));
        assert!(parse_collection_id("hello").is_none());
        let json: serde_json::Value = serde_json::from_str(
            r#"{"response":{"collectiondetails":[{"children":[
                {"publishedfileid":"2","sortorder":2},{"publishedfileid":"1","sortorder":1}]}]}}"#,
        )
        .unwrap();
        assert_eq!(parse_collection_children(&json), vec!["1", "2"]);
    }

    #[test]
    fn parses_details_response() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"response":{"result":1,"resultcount":2,"publishedfiledetails":[
              {"publishedfileid":"1","result":1,"title":"A","description":"d","time_created":1,"time_updated":2,"preview_url":"p","tags":[{"tag":"units"}]},
              {"publishedfileid":"2","result":9}
            ]}}"#,
        )
        .unwrap();
        let items = parse_details(&json);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "A");
        assert_eq!(items[0].tags, vec!["units"]);
        assert_eq!(items[0].time_updated, 2);
    }

    #[test]
    fn parses_required_items_block() {
        let html = r#"<div class="requiredItemsContainer" id="RequiredItems">
          <a href="https://steamcommunity.com/workshop/filedetails/?id=111" target="_blank"><div class="requiredItem">X</div></a>
          <a href="https://steamcommunity.com/workshop/filedetails/?id=222"><div>Y</div></a>
        </div><a href="https://steamcommunity.com/sharedfiles/filedetails/?id=333">unrelated</a>"#;
        assert_eq!(parse_required_items(html, "999"), vec!["111", "222", "333"]);
        assert!(parse_required_items("<html></html>", "1").is_empty());
    }
}
