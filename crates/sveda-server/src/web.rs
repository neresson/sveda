use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT_ENCODING, CONTENT_TYPE, LOCATION, USER_AGENT,
};
use reqwest::redirect::Policy;
use reqwest::{Client, Url};
use serde_json::{json, Value};
use sveda_mcp::McpTool;

use crate::Config;

pub const SEARCH_TOOL_NAME: &str = "web_search";
pub const FETCH_TOOL_NAME: &str = "web_fetch";

const FETCH_UA: &str = "SvedaBot/0.2 (+https://sveda.dev)";
const SEARCH_UA: &str =
    "Mozilla/5.0 (compatible; SvedaBot/0.2; +https://sveda.dev) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";
const MAX_BODY_BYTES: usize = 1_500_000;
const MAX_REDIRECTS: usize = 5;
const DEFAULT_MAX_CHARS: usize = 24_000;
const DDG_SEARCH_URL: &str = "https://html.duckduckgo.com/html/";
const BING_SEARCH_URL: &str = "https://www.bing.com/search";

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub timeout: u64,
    pub max_chars: usize,
    pub searxng_url: String,
    pub allow_private: bool,
}

impl WebConfig {
    pub fn from_runtime(config: &Config) -> Self {
        Self {
            timeout: config.web_timeout.max(1),
            max_chars: DEFAULT_MAX_CHARS,
            searxng_url: config.searxng_url.clone(),
            allow_private: false,
        }
    }
}

pub fn tools() -> Vec<McpTool> {
    vec![
        tool(
            SEARCH_TOOL_NAME,
            "Search the public internet. Use this for current events, facts you are unsure about, documentation, news, prices, and anything that may have changed. Returns titles, URLs, and snippets. Follow up with web_fetch on the most relevant results.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of results to return (1-12). Default 8."
                    }
                },
                "required": ["query"]
            }),
        ),
        tool(
            FETCH_TOOL_NAME,
            "Fetch a public http(s) URL and extract readable text. Use after web_search, or when the user provides a link. Do not fetch private, local, or metadata addresses.",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Absolute http or https URL"
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "Maximum characters of extracted text to return"
                    }
                },
                "required": ["url"]
            }),
        ),
    ]
}

pub fn names() -> Vec<String> {
    tools().into_iter().map(|tool| tool.name).collect()
}

pub fn instructions() -> String {
    format!(
        "You have internet tools: {}. Use them to look up current events, facts you are unsure about, documentation, prices, news, and anything that may have changed. Prefer web_search first, then web_fetch on the most relevant URLs. Independent searches can run in parallel with spawn_tasks. Cite sources with their URLs. Do not claim you cannot browse or search the web.",
        names().join(", ")
    )
}

pub fn handler(config: WebConfig) -> sveda_agent::NativeHandler {
    let http = build_client(&config);
    Arc::new(move |name, input| {
        let http = http.clone();
        let config = config.clone();
        Box::pin(async move { execute(&http, &config, &name, input).await })
    })
}

pub async fn execute(http: &Client, config: &WebConfig, name: &str, input: Value) -> Value {
    match name {
        SEARCH_TOOL_NAME => search(http, config, &input).await,
        FETCH_TOOL_NAME => fetch(http, config, &input).await,
        _ => fail(&format!("Unknown web tool {name}.")),
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> McpTool {
    McpTool {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
        domain: "web".into(),
        mode: "read".into(),
        confirmation_required: false,
    }
}

fn build_client(config: &WebConfig) -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    Client::builder()
        .timeout(Duration::from_secs(config.timeout.max(1)))
        .connect_timeout(Duration::from_secs(config.timeout.min(10).max(1)))
        .redirect(Policy::none())
        .default_headers(headers)
        .build()
        .expect("reqwest client")
}

#[derive(Clone, Debug)]
struct SearchHit {
    title: String,
    url: String,
    snippet: String,
}

async fn search(http: &Client, config: &WebConfig, input: &Value) -> Value {
    let Some(query) = string_field(input, "query") else {
        return fail("Query is required.");
    };
    let count = input
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 12) as usize;

    let mut last_error = String::from("No web results.");
    if !config.searxng_url.trim().is_empty() {
        match searxng_search(http, config, &query, count).await {
            Ok(hits) if !hits.is_empty() => return search_ok("searxng", &query, hits),
            Ok(_) => last_error = "SearXNG returned no results.".into(),
            Err(error) => last_error = error,
        }
    }
    match duckduckgo_search(http, &query, count).await {
        Ok(hits) if !hits.is_empty() => return search_ok("duckduckgo", &query, hits),
        Ok(_) => {}
        Err(error) => last_error = error,
    }
    match bing_search(http, &query, count).await {
        Ok(hits) if !hits.is_empty() => return search_ok("bing", &query, hits),
        Ok(_) => fail("No web results."),
        Err(_) => fail(&last_error),
    }
}

async fn fetch(http: &Client, config: &WebConfig, input: &Value) -> Value {
    let Some(raw) = string_field(input, "url") else {
        return fail("URL is required.");
    };
    let max_chars = input
        .get("max_chars")
        .and_then(Value::as_u64)
        .map(|value| value.clamp(1, 80_000) as usize)
        .unwrap_or(config.max_chars.max(1));

    let mut url = match parse_http_url(&raw) {
        Ok(url) => url,
        Err(error) => return fail(&error),
    };
    if let Err(error) = ensure_public_url(&url, config.allow_private).await {
        return fail(&error);
    }

    let mut response = None;
    for _ in 0..=MAX_REDIRECTS {
        let sent = match http
            .get(url.clone())
            .header(USER_AGENT, FETCH_UA)
            .header("Accept", "text/html,application/xhtml+xml,application/xml,application/json,text/plain;q=0.9,*/*;q=0.8")
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => return fail(&format!("Could not fetch URL: {error}")),
        };
        if sent.status().is_redirection() {
            let Some(location) = sent
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                return fail("Redirect was missing a Location header.");
            };
            url = match url.join(location).or_else(|_| Url::parse(location)) {
                Ok(next) => next,
                Err(_) => return fail("Redirect Location was not a valid URL."),
            };
            if let Err(error) = parse_http_url(url.as_str()).map(|_| ()) {
                return fail(&error);
            }
            if let Err(error) = ensure_public_url(&url, config.allow_private).await {
                return fail(&error);
            }
            continue;
        }
        response = Some(sent);
        break;
    }
    let Some(response) = response else {
        return fail("Too many redirects.");
    };
    let status = response.status();
    if !status.is_success() {
        return fail(&format!("Fetch failed with HTTP {}.", status.as_u16()));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return fail(&format!("Could not read response body: {error}")),
    };
    if bytes.len() > MAX_BODY_BYTES {
        return fail("Page is larger than the fetch limit.");
    }
    let lower_type = content_type.to_ascii_lowercase();
    if lower_type.contains("image/")
        || lower_type.contains("audio/")
        || lower_type.contains("video/")
        || lower_type.contains("application/pdf")
        || lower_type.contains("application/octet-stream")
        || lower_type.contains("application/zip")
    {
        return fail("This content type cannot be extracted as text.");
    }
    let body = String::from_utf8_lossy(&bytes).to_string();
    let (title, text) = extract_document(&body, &lower_type);
    let truncated = text.chars().count() > max_chars;
    let clipped: String = text.chars().take(max_chars).collect();
    let title = if title.trim().is_empty() {
        url.host_str().unwrap_or("page").to_string()
    } else {
        title
    };
    let final_url = serialize_http_url(&url);
    let links = json!([{
        "label": title,
        "url": final_url,
        "kind": "reference"
    }]);
    json!({
        "success": true,
        "data": {
            "url": final_url,
            "title": title,
            "content_type": content_type,
            "text": clipped,
            "truncated": truncated
        },
        "links": links
    })
}

async fn searxng_search(
    http: &Client,
    config: &WebConfig,
    query: &str,
    count: usize,
) -> Result<Vec<SearchHit>, String> {
    let url = searxng_search_url(&config.searxng_url, query)?;
    let response = http
        .get(url)
        .header(USER_AGENT, FETCH_UA)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("SearXNG request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "SearXNG failed with HTTP {}.",
            response.status().as_u16()
        ));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|error| format!("SearXNG returned invalid JSON: {error}"))?;
    Ok(parse_searxng_json(&payload, count))
}

async fn duckduckgo_search(
    http: &Client,
    query: &str,
    count: usize,
) -> Result<Vec<SearchHit>, String> {
    let mut url = Url::parse(DDG_SEARCH_URL).expect("ddg url");
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("kl", "wt-wt");
    let html = get_html(http, url, SEARCH_UA).await?;
    Ok(parse_duckduckgo_html(&html, count))
}

async fn bing_search(http: &Client, query: &str, count: usize) -> Result<Vec<SearchHit>, String> {
    let mut url = Url::parse(BING_SEARCH_URL).expect("bing url");
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("setlang", "en");
    let html = get_html(http, url, SEARCH_UA).await?;
    Ok(parse_bing_html(&html, count))
}

async fn get_html(http: &Client, url: Url, ua: &str) -> Result<String, String> {
    let response = http
        .get(url)
        .header(USER_AGENT, ua)
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|error| format!("Search request failed: {error}"))?;
    if !response.status().is_success() && response.status().as_u16() != 202 {
        return Err(format!(
            "Search failed with HTTP {}.",
            response.status().as_u16()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Search response could not be read: {error}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn searxng_search_url(base: &str, query: &str) -> Result<Url, String> {
    let mut url = operator_http_url(base)?;
    let path = url.path().trim_end_matches('/');
    if !path.ends_with("/search") {
        url.set_path(&format!("{path}/search"));
    }
    url.query_pairs_mut()
        .clear()
        .append_pair("q", query)
        .append_pair("format", "json")
        .append_pair("categories", "general");
    Ok(url)
}

fn operator_http_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|_| "SearXNG URL is invalid.".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("SearXNG URL must be http or https.".into());
    }
    if url.host_str().is_none() {
        return Err("SearXNG URL host is required.".into());
    }
    Ok(url)
}

fn parse_searxng_json(payload: &Value, count: usize) -> Vec<SearchHit> {
    payload
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            hit_from(
                item.get("title").and_then(Value::as_str).unwrap_or(""),
                item.get("url")
                    .or_else(|| item.get("pretty_url"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                item.get("content")
                    .or_else(|| item.get("snippet"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        })
        .take(count)
        .collect()
}

fn parse_duckduckgo_html(html: &str, count: usize) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut rest = html;
    while hits.len() < count {
        let Some(marker) = rest.find("result__a") else {
            break;
        };
        let slice = &rest[find_last_open_before(rest, marker).unwrap_or(marker)..];
        let href = extract_attr(slice, "href").unwrap_or_default();
        let title = strip_tags(&inner_until(slice, "</a>").unwrap_or_default());
        let after_link = slice
            .find("</a>")
            .map(|index| &slice[index + 4..])
            .unwrap_or("");
        let snippet = snippet_from_ddg(after_link);
        rest = &rest[marker + 9..];
        let url = unwrap_ddg_url(&href).unwrap_or(href);
        if let Some(hit) = hit_from(&title, &url, &snippet) {
            hits.push(hit);
        }
    }
    hits
}

fn parse_bing_html(html: &str, count: usize) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut rest = html;
    while hits.len() < count {
        let Some(marker) = rest.find("b_algo") else {
            break;
        };
        let block_start = find_last_open_before(rest, marker).unwrap_or(marker);
        let block_end = rest[block_start..]
            .find("</li>")
            .map(|index| block_start + index)
            .unwrap_or(rest.len());
        let block = &rest[block_start..block_end];
        rest = &rest[block_end.min(rest.len()).saturating_add(1).min(rest.len())..];
        if !block.contains("b_algo") {
            continue;
        }
        let heading = inner_of(block, "h2").unwrap_or_default();
        let href = extract_attr(&heading, "href")
            .or_else(|| extract_attr(block, "href"))
            .unwrap_or_default();
        let title = collapse_ws(&strip_tags(&heading));
        let snippet = inner_of(block, "p")
            .or_else(|| inner_of(block, "span"))
            .map(|value| collapse_ws(&strip_tags(&value)))
            .unwrap_or_default();
        let url = decode_bing_url(&href).unwrap_or(href);
        if let Some(hit) = hit_from(&title, &url, &snippet) {
            hits.push(hit);
        }
    }
    hits
}

fn snippet_from_ddg(after_link: &str) -> String {
    if let Some(index) = after_link.find("result__snippet") {
        let slice = &after_link[index..];
        return collapse_ws(&strip_tags(
            &inner_until(slice, "</a>")
                .or_else(|| inner_until(slice, "</td>"))
                .unwrap_or_default(),
        ));
    }
    String::new()
}

fn hit_from(title: &str, url: &str, snippet: &str) -> Option<SearchHit> {
    let title = collapse_ws(&decode_entities(title));
    let snippet = collapse_ws(&decode_entities(snippet));
    let url = normalize_result_url(url)?;
    if title.is_empty() || is_search_interstitial(&url) {
        return None;
    }
    Some(SearchHit {
        title,
        url,
        snippet,
    })
}

fn normalize_result_url(href: &str) -> Option<String> {
    let href = decode_entities(href.trim());
    if href.is_empty() {
        return None;
    }
    if let Some(url) = decode_bing_url(&href) {
        return http_url_string(&url);
    }
    if let Some(url) = unwrap_ddg_url(&href) {
        return http_url_string(&url);
    }
    http_url_string(&href)
}

fn http_url_string(raw: &str) -> Option<String> {
    let url = parse_http_url(raw).ok()?;
    let host = url.host_str()?;
    if is_blocked_host(host) {
        return None;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return None;
        }
    }
    Some(serialize_http_url(&url))
}

fn serialize_http_url(url: &Url) -> String {
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        let mut serialized = url.to_string();
        if serialized.ends_with('/') {
            serialized.pop();
        }
        serialized
    } else {
        url.to_string()
    }
}

fn unwrap_ddg_url(href: &str) -> Option<String> {
    let href = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };
    let url = Url::parse(&href).ok()?;
    url.query_pairs()
        .find(|(key, _)| key == "uddg")
        .map(|(_, value)| value.into_owned())
}

fn decode_bing_url(href: &str) -> Option<String> {
    let href = decode_entities(href);
    let raw = bing_u_param(&href)?;
    let payload = raw.strip_prefix("a1").unwrap_or(&raw);
    let bytes = decode_padded_b64(payload)?;
    let decoded = String::from_utf8(bytes).ok()?;
    if decoded.starts_with("http://") || decoded.starts_with("https://") {
        Some(decoded)
    } else {
        None
    }
}

fn bing_u_param(href: &str) -> Option<String> {
    if let Ok(url) = Url::parse(href) {
        if let Some((_, value)) = url.query_pairs().find(|(key, _)| key == "u") {
            return Some(value.into_owned());
        }
    }
    let marker = href.find("&u=").or_else(|| href.find("?u="))?;
    let rest = &href[marker + 3..];
    Some(rest.split('&').next()?.to_string())
}

fn decode_padded_b64(raw: &str) -> Option<Vec<u8>> {
    let mut padded = raw.to_string();
    let pad = (4 - padded.len() % 4) % 4;
    padded.extend(std::iter::repeat('=').take(pad));
    base64::engine::general_purpose::URL_SAFE
        .decode(padded.as_bytes())
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(padded.as_bytes()))
        .ok()
}

fn is_search_interstitial(url: &str) -> bool {
    url.contains("bing.com/ck/")
        || url.contains("bing.com/search")
        || url.contains("duckduckgo.com/l/")
        || url.contains("duckduckgo.com/y.js")
}

fn parse_http_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|_| "URL is invalid.".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Only http and https URLs are allowed.".into());
    }
    if url.host_str().is_none() {
        return Err("URL host is required.".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URLs with credentials are not allowed.".into());
    }
    Ok(url)
}

async fn ensure_public_url(url: &Url, allow_private: bool) -> Result<(), String> {
    if allow_private {
        return Ok(());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "URL host is required.".to_string())?;
    if is_blocked_host(host) {
        return Err("This URL is not allowed.".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err("This URL is not allowed.".into());
        }
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "Could not resolve URL host.".to_string())?;
    let mut resolved = false;
    for addr in addrs {
        resolved = true;
        if is_blocked_ip(addr.ip()) {
            return Err("This URL is not allowed.".into());
        }
    }
    if !resolved {
        return Err("Could not resolve URL host.".into());
    }
    Ok(())
}

fn is_blocked_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".lan")
        || host == "metadata.google.internal"
        || host == "kubernetes"
        || host == "kubernetes.default"
        || host == "kubernetes.default.svc"
        || host == "kubernetes.default.svc.cluster.local"
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_v4(ip),
        IpAddr::V6(ip) => is_blocked_v6(ip),
    }
}

fn is_blocked_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..128).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
}

fn is_blocked_v6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_blocked_v4(mapped);
    }
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
}

fn extract_document(body: &str, content_type: &str) -> (String, String) {
    if content_type.contains("application/json") || looks_like_json(body) {
        let pretty = serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
            .unwrap_or_else(|| collapse_ws(body));
        return ("JSON".into(), pretty);
    }
    if content_type.contains("text/plain")
        && !content_type.contains("html")
        && !looks_like_html(body)
    {
        return (String::new(), collapse_ws(body));
    }
    let title = inner_of(body, "title")
        .map(|value| collapse_ws(&decode_entities(&strip_tags(&value))))
        .unwrap_or_default();
    (title, html_to_text(body))
}

fn looks_like_json(body: &str) -> bool {
    let trimmed = body.trim_start();
    (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<Value>(body).is_ok()
}

fn looks_like_html(body: &str) -> bool {
    let lower = body.trim_start().to_ascii_lowercase();
    lower.starts_with("<!doctype html") || lower.starts_with("<html")
}

fn html_to_text(html: &str) -> String {
    let mut text = remove_blocks(
        html.to_string(),
        &["script", "style", "noscript", "svg", "iframe", "template"],
    );
    for (from, to) in [
        ("<br>", "\n"),
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("</p>", "\n\n"),
        ("</div>", "\n"),
        ("</h1>", "\n\n"),
        ("</h2>", "\n\n"),
        ("</h3>", "\n\n"),
        ("</h4>", "\n\n"),
        ("</li>", "\n"),
        ("</tr>", "\n"),
        ("</table>", "\n"),
    ] {
        text = replace_ci(&text, from, to);
    }
    collapse_ws(&decode_entities(&strip_tags(&text)))
}

fn remove_blocks(mut html: String, tags: &[&str]) -> String {
    for tag in tags {
        loop {
            let lower = html.to_ascii_lowercase();
            let open = format!("<{tag}");
            let Some(start) = lower.find(&open) else {
                break;
            };
            let close = format!("</{tag}>");
            let end = lower[start..]
                .find(&close)
                .map(|index| start + index + close.len())
                .unwrap_or(html.len());
            html.replace_range(start..end, " ");
        }
    }
    html
}

fn replace_ci(input: &str, from: &str, to: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let needle = from.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;
    while let Some(found) = lower[index..].find(&needle) {
        let at = index + found;
        out.push_str(&input[index..at]);
        out.push_str(to);
        index = at + from.len();
    }
    out.push_str(&input[index..]);
    out
}

fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        if is_plausible_entity(entity) {
            if let Some(ch) = decode_entity(entity) {
                out.push(ch);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn is_plausible_entity(entity: &str) -> bool {
    !entity.is_empty()
        && entity.len() <= 8
        && entity
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '#')
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" | "#039" => Some('\''),
        "nbsp" | "#160" | "#0160" => Some(' '),
        other if other.starts_with("#x") || other.starts_with("#X") => {
            u32::from_str_radix(&other[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        other if other.starts_with('#') => other[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn collapse_ws(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut newline_run = 0;
    let mut space = false;
    for ch in input.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            space = false;
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
            continue;
        }
        newline_run = 0;
        if ch.is_whitespace() {
            if !space && !out.ends_with('\n') && !out.is_empty() {
                out.push(' ');
                space = true;
            }
            continue;
        }
        space = false;
        out.push(ch);
    }
    out.trim().to_string()
}

fn extract_attr(fragment: &str, name: &str) -> Option<String> {
    let lower = fragment.to_ascii_lowercase();
    let needle = format!("{name}=");
    let index = lower.find(&needle)?;
    let rest = fragment[index + needle.len()..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

fn inner_of(hay: &str, tag: &str) -> Option<String> {
    let lower = hay.to_ascii_lowercase();
    let open = format!("<{tag}");
    let start = lower.find(&open)?;
    let after = hay[start..].find('>')? + start + 1;
    let close = format!("</{tag}>");
    let end = lower[after..].find(&close)? + after;
    Some(hay[after..end].to_string())
}

fn inner_until(hay: &str, close: &str) -> Option<String> {
    let start = hay.find('>')? + 1;
    let lower = hay.to_ascii_lowercase();
    let end = lower[start..].find(&close.to_ascii_lowercase())? + start;
    Some(hay[start..end].to_string())
}

fn find_last_open_before(hay: &str, index: usize) -> Option<usize> {
    hay[..index].rfind('<')
}

fn string_field(input: &Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn search_ok(backend: &str, query: &str, hits: Vec<SearchHit>) -> Value {
    let links: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "label": hit.title,
                "url": hit.url,
                "kind": "reference"
            })
        })
        .collect();
    let results: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "title": hit.title,
                "url": hit.url,
                "snippet": hit.snippet
            })
        })
        .collect();
    json!({
        "success": true,
        "data": {
            "query": query,
            "backend": backend,
            "results": results
        },
        "links": links
    })
}

fn fail(message: &str) -> Value {
    json!({ "success": false, "error": message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn config_for(server: &MockServer) -> (Client, WebConfig) {
        let config = WebConfig {
            timeout: 5,
            max_chars: 400,
            searxng_url: server.uri(),
            allow_private: true,
        };
        (build_client(&config), config)
    }

    #[test]
    fn catalog_covers_search_and_fetch() {
        let names = names();
        assert_eq!(names, vec!["web_search", "web_fetch"]);
        for tool in tools() {
            assert_eq!(tool.domain, "web");
            assert_eq!(tool.mode, "read");
            assert!(!tool.description.is_empty());
        }
        let text = instructions();
        assert!(text.contains("web_search"));
        assert!(text.contains("web_fetch"));
        assert!(text.contains("Do not claim you cannot browse"));
    }

    #[test]
    fn bing_html_decodes_destination_urls() {
        let html = r#"
            <li class="b_algo">
                <h2><a href="https://www.bing.com/ck/a?!&&p=abc&amp;u=a1aHR0cHM6Ly9leGFtcGxlLmNvbS9kb2Nz">Example Docs</a></h2>
                <p>Official documentation snippet.</p>
            </li>
            <li class="b_algo">
                <h2><a href="https://www.bing.com/ck/a?!&&p=def&u=a1aHR0cHM6Ly9lbi53aWtpcGVkaWEub3JnL3dpa2kvUnVzdA">Rust - Wikipedia</a></h2>
                <p>Rust is a language.</p>
            </li>
        "#;
        let hits = parse_bing_html(html, 8);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Example Docs");
        assert_eq!(hits[0].url, "https://example.com/docs");
        assert!(hits[0].snippet.contains("documentation"));
        assert_eq!(hits[1].url, "https://en.wikipedia.org/wiki/Rust");
    }

    #[test]
    fn search_hits_drop_private_and_loopback_urls() {
        assert!(hit_from("Secret", "http://127.0.0.1/admin", "nope").is_none());
        assert!(hit_from("Meta", "http://169.254.169.254/", "nope").is_none());
        assert!(hit_from("Local", "http://localhost/docs", "nope").is_none());
        assert!(hit_from("Sveda", "https://sveda.dev/docs", "ok").is_some());
    }

    #[test]
    fn searxng_url_appends_search_json() {
        let url = searxng_search_url("https://searx.example", "sveda agent").unwrap();
        assert_eq!(url.path(), "/search");
        let query: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        assert!(query.iter().any(|(k, v)| k == "q" && v == "sveda agent"));
        assert!(query.iter().any(|(k, v)| k == "format" && v == "json"));
    }

    #[test]
    fn duckduckgo_html_unwraps_uddg() {
        let html = r##"
            <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust-lang.org%2Fbook%2F">The Rust Book</a>
            <a class="result__snippet" href="#">The official book.</a>
        "##;
        let hits = parse_duckduckgo_html(html, 4);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "The Rust Book");
        assert_eq!(hits[0].url, "https://doc.rust-lang.org/book/");
        assert!(hits[0].snippet.contains("official book"));
    }

    #[test]
    fn json_backends_parse_results() {
        let searx = json!({
            "results": [
                { "title": "Docs", "url": "https://sveda.dev/docs", "content": "Protocol docs." }
            ]
        });
        let hits = parse_searxng_json(&searx, 8);
        assert_eq!(hits[0].title, "Docs");
        assert_eq!(hits[0].snippet, "Protocol docs.");
    }

    #[test]
    fn html_to_text_strips_chrome_and_keeps_copy() {
        let html = r#"
            <html><head><title>Hello &amp; Co</title><script>secret()</script></head>
            <body>
                <h1>Welcome</h1>
                <p>First paragraph.</p>
                <style>.x{}</style>
                <p>Second&nbsp;line.</p>
            </body></html>
        "#;
        let (title, text) = extract_document(html, "text/html");
        assert_eq!(title, "Hello & Co");
        assert!(text.contains("Welcome"));
        assert!(text.contains("First paragraph."));
        assert!(text.contains("Second line."));
        assert!(!text.contains("secret"));
        assert!(!text.contains(".x{}"));
    }

    #[test]
    fn blocked_hosts_and_ips() {
        assert!(is_blocked_host("localhost"));
        assert!(is_blocked_host("foo.localhost"));
        assert!(is_blocked_host("metadata.google.internal"));
        assert!(is_blocked_host("svc.internal"));
        assert!(!is_blocked_host("example.com"));
        assert!(is_blocked_ip("127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ip("10.0.0.8".parse().unwrap()));
        assert!(is_blocked_ip("192.168.1.1".parse().unwrap()));
        assert!(is_blocked_ip("169.254.169.254".parse().unwrap()));
        assert!(is_blocked_ip("::1".parse().unwrap()));
        assert!(is_blocked_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_blocked_ip("1.1.1.1".parse().unwrap()));
        assert!(parse_http_url("file:///etc/passwd").is_err());
        assert!(parse_http_url("http://user:pass@example.com").is_err());
    }

    #[tokio::test]
    async fn fetch_rejects_loopback_without_override() {
        let config = WebConfig {
            timeout: 2,
            max_chars: 100,
            searxng_url: String::new(),
            allow_private: false,
        };
        let http = build_client(&config);
        let output = execute(
            &http,
            &config,
            FETCH_TOOL_NAME,
            json!({ "url": "http://127.0.0.1/" }),
        )
        .await;
        assert_eq!(output["success"], false);
        assert!(output["error"].as_str().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn fetch_extracts_html_from_mock() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "<html><head><title>Mock Page</title></head><body><p>Readable copy.</p></body></html>",
                "text/html",
            ))
            .mount(&server)
            .await;
        let (http, config) = config_for(&server);
        let output = execute(
            &http,
            &config,
            FETCH_TOOL_NAME,
            json!({ "url": format!("{}/page", server.uri()) }),
        )
        .await;
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["title"], "Mock Page");
        assert!(output["data"]["text"]
            .as_str()
            .unwrap()
            .contains("Readable copy."));
        assert_eq!(output["links"][0]["kind"], "reference");
    }

    #[tokio::test]
    async fn fetch_follows_relative_redirect() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/go"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", "/page"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "<html><head><title>Redirected</title></head><body><p>Landed.</p></body></html>",
                "text/html",
            ))
            .mount(&server)
            .await;
        let (http, config) = config_for(&server);
        let output = execute(
            &http,
            &config,
            FETCH_TOOL_NAME,
            json!({ "url": format!("{}/go", server.uri()) }),
        )
        .await;
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["title"], "Redirected");
        assert!(output["data"]["text"].as_str().unwrap().contains("Landed."));
        assert!(output["data"]["url"].as_str().unwrap().ends_with("/page"));
    }

    #[tokio::test]
    async fn redirect_to_loopback_is_blocked() {
        let next = Url::parse("https://example.com/go")
            .unwrap()
            .join("http://127.0.0.1/secret")
            .unwrap();
        let error = ensure_public_url(&next, false).await.unwrap_err();
        assert!(error.contains("not allowed"));
    }

    #[tokio::test]
    async fn search_uses_searxng_when_configured() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "sveda agent"))
            .and(query_param("format", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {
                        "title": "Sveda",
                        "url": "https://sveda.dev",
                        "content": "Open-source embeddable agent."
                    }
                ]
            })))
            .mount(&server)
            .await;
        let (http, config) = config_for(&server);
        let output = execute(
            &http,
            &config,
            SEARCH_TOOL_NAME,
            json!({ "query": "sveda agent", "count": 3 }),
        )
        .await;
        assert_eq!(output["success"], true);
        assert_eq!(output["data"]["backend"], "searxng");
        assert_eq!(output["data"]["results"][0]["url"], "https://sveda.dev");
        assert_eq!(output["links"][0]["label"], "Sveda");
    }

    #[tokio::test]
    async fn search_requires_query() {
        let config = WebConfig {
            timeout: 1,
            max_chars: 10,
            searxng_url: String::new(),
            allow_private: true,
        };
        let http = build_client(&config);
        let output = execute(&http, &config, SEARCH_TOOL_NAME, json!({})).await;
        assert_eq!(output["error"], "Query is required.");
    }
}
