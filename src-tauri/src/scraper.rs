//! Metadata scraper for supported storefronts.
//! Supported: Booth, Gumroad, Jinxxy, Payhip.
//!
//! Preference order for every field: schema.org `Product` JSON-LD (a
//! structured data block most storefronts embed for search engines) first,
//! since it's far less brittle than guessing CSS class names; then Open
//! Graph tags; then site-specific tag scanning as a last resort.
//!
//! Images: `collect_images` merges JSON-LD `image`, every `og:image` tag
//! (some storefronts emit one per gallery photo), and a generic gallery scan
//! — then deduplicates by a normalized key so the same photo served at two
//! different resize/CDN variants doesn't show up twice. Booth additionally
//! gets a dedicated, markup-verified gallery extractor (see
//! `booth_gallery_images`) since its resize-variant URLs would otherwise
//! collide under naive string matching.

use serde::Serialize;
use serde_json::Value as Json;
use std::collections::HashSet;

/// Cap on how many images we'll ever return for one page, so a pathological
/// page (or an over-eager gallery match) can't blow up the import step.
const MAX_IMAGES: usize = 24;

/// The subset of metadata we can extract from a product page.
#[derive(Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScrapedMeta {
    /// Product/item name.
    pub name: Option<String>,
    /// Creator/author name as it appears on the page.
    pub creator_name: Option<String>,
    /// Absolute URL of the creator/shop icon, if one could be found.
    pub creator_icon_url: Option<String>,
    /// The creator's own shop/profile page, if we could determine one —
    /// used to pre-fill "Profile links" when auto-creating a new creator.
    pub creator_profile_url: Option<String>,
    /// Every product image we could find on the page, in page order,
    /// deduplicated. May be empty.
    pub image_urls: Vec<String>,
    /// Human-readable site name (e.g. "Booth", "Gumroad").
    pub site_name: Option<String>,
}

/// Returns the display name of the supported site for a given URL,
/// or None if the URL's domain isn't in the supported list.
pub fn detect_site(url: &str) -> Option<&'static str> {
    let lower = url.to_lowercase();
    if lower.contains("booth.pm") {
        Some("Booth")
    } else if lower.contains("gumroad.com") {
        Some("Gumroad")
    } else if lower.contains("jinxxy.com") {
        Some("Jinxxy")
    } else if lower.contains("payhip.com") {
        Some("Payhip")
    } else {
        None
    }
}

/// Scrapes metadata from a supported product page URL.
/// Returns an error string if the fetch fails or the URL isn't supported.
pub async fn scrape(url: &str) -> Result<ScrapedMeta, String> {
    let site = detect_site(url)
        .ok_or_else(|| format!("'{url}' is not a supported site"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Could not fetch '{url}': {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "'{url}' returned status {}",
            resp.status()
        ));
    }

    // The response URL (after following any redirect) is a more accurate
    // base than the URL we started from, e.g. if the site issues an
    // http->https or www-normalizing redirect.
    let final_url = resp.url().to_string();

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Could not read response from '{url}': {e}"))?;

    let mut scraped = match site {
        "Booth" => scrape_booth(&html),
        "Gumroad" => scrape_gumroad(&html),
        "Jinxxy" => scrape_jinxxy(&html),
        "Payhip" => scrape_payhip(&html),
        _ => scrape_og(&html, site),
    };

    // Booth, Gumroad and Payhip each give a creator their own shop
    // subdomain (e.g. `lilxyzw.booth.pm`), so the origin of the URL we
    // fetched is usually the creator's profile page — this only breaks
    // for Booth when the *fetched* URL was itself the shared, non-subdomain
    // form (`booth.pm/<locale>/items/<id>`), which `scrape_booth` handles
    // separately by reading the shop's real subdomain straight off the
    // page's own nav link. Jinxxy is a shared marketplace domain with
    // per-creator paths rather than subdomains, so its origin wouldn't
    // point at anything creator-specific — leave it unset there.
    if matches!(site, "Booth" | "Gumroad" | "Payhip") {
        scraped.creator_profile_url = scraped.creator_profile_url.or_else(|| origin_of(&final_url));
    }

    Ok(scraped)
}

/// Returns the `scheme://host[:port]` portion of an absolute URL.
fn origin_of(url: &str) -> Option<String> {
    let scheme_end = url.find("://")? + 3;
    let rest = &url[scheme_end..];
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if host_end == 0 {
        return None;
    }
    Some(url[..scheme_end + host_end].to_string())
}

// ---------------------------------------------------------------------------
// Low-level helpers
// ---------------------------------------------------------------------------

/// ASCII case-insensitive substring search that returns a byte offset
/// valid within `haystack` itself. Every needle used in this module is
/// plain ASCII (tag/attribute names), so this scans raw bytes rather than
/// comparing against a `to_lowercase()` copy: Unicode case folding isn't
/// guaranteed to preserve a character's byte length (e.g. Turkish `İ`
/// lowercases to two code points), which would misalign an offset found in
/// a lowercased copy against the original string and risk slicing it —
/// real product pages here are full of multi-byte Japanese/Unicode text —
/// on a non-character boundary. Because ASCII bytes never occur inside a
/// multi-byte UTF-8 sequence, a byte-for-byte match here can only ever
/// land on a real character boundary.
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return Some(0);
    }
    if n.len() > h.len() {
        return None;
    }
    (0..=h.len() - n.len()).find(|&start| h[start..start + n.len()].eq_ignore_ascii_case(n))
}

/// Same as `find_ci` but returns the *last* match — for the same reason,
/// never derive this from a `to_lowercase()`-ed copy's `rfind`.
fn rfind_ci(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return Some(h.len());
    }
    if n.len() > h.len() {
        return None;
    }
    (0..=h.len() - n.len()).rev().find(|&start| h[start..start + n.len()].eq_ignore_ascii_case(n))
}

/// Largest index `<= idx` (clamped to `s.len()`) that lands on a UTF-8
/// character boundary of `s`. Used to build a lookback window ending at a
/// tag's start without risking a slice that splits a multi-byte character.
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Pulls the value of an HTML attribute from a tag fragment.
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let needle_dq = format!(r#"{attr}=""#);
    let needle_sq = format!(r#"{attr}='"#);
    if let Some(pos) = find_ci(tag, &needle_dq) {
        let rest = &tag[pos + needle_dq.len()..];
        let end = rest.find('"').unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    if let Some(pos) = find_ci(tag, &needle_sq) {
        let rest = &tag[pos + needle_sq.len()..];
        let end = rest.find('\'').unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

/// Minimal HTML entity decoder for the characters we commonly see in titles.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Pulls text between two substrings.
fn between<'a>(html: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = find_ci(html, start)?;
    let rest = &html[s + start.len()..];
    let e = rest.find(end)?;
    Some(&rest[..e])
}

/// Strips HTML tags from a short fragment and trims the result.
fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(out.trim())
}

/// Returns the (start, end) byte range — end exclusive of '>' — of every
/// occurrence of a tag (e.g. "<meta", "<img") in the document, in order.
fn find_tags(html: &str, tag: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < html.len() {
        let Some(rel) = find_ci(&html[idx..], tag) else { break };
        let start = idx + rel;
        let end = html[start..].find('>').map(|i| start + i).unwrap_or(html.len());
        out.push((start, end));
        idx = end.max(start + 1);
    }
    out
}

/// Extracts the content of the first meta tag matching a property/name value.
fn meta(html: &str, attr: &str, value: &str) -> Option<String> {
    meta_all(html, attr, value).into_iter().next()
}

/// Extracts the content of every meta tag matching a property/name value,
/// in document order. Handles both attribute orders and quote styles.
fn meta_all(html: &str, attr: &str, value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let prop_dq = format!(r#"{attr}="{value}""#).to_lowercase();
    let prop_sq = format!(r#"{attr}='{value}'"#).to_lowercase();

    for (start, end) in find_tags(html, "<meta") {
        let tag = &html[start..end];
        let tag_lower = tag.to_lowercase();
        if tag_lower.contains(&prop_dq) || tag_lower.contains(&prop_sq) {
            if let Some(c) = extract_attr(tag, "content") {
                let decoded = decode_entities(&c);
                if !decoded.is_empty() {
                    out.push(decoded);
                }
            }
        }
    }
    out
}

/// Finds the first absolute URL on the page that contains `needle` anywhere
/// in it, regardless of whether it sits in an `href`, `src`, a `data-*`
/// attribute, an inline `style="background-image:url(...)"`, or embedded
/// JSON — this is far more resilient to markup changes than matching a
/// specific tag/attribute/class combination. Used for CDN paths with a
/// distinctive, stable segment (e.g. Booth's `/icon_image/`).
fn find_url_containing(html: &str, needle: &str) -> Option<String> {
    let needle_lower = needle.to_lowercase();
    let mut idx = 0usize;
    while let Some(rel) = html[idx..].find("https://") {
        let start = idx + rel;
        let rest = &html[start..];
        let end = rest
            .find(|c: char| matches!(c, '"' | '\'' | ')' | ' ' | '<' | '>' | '\\'))
            .unwrap_or(rest.len());
        let url = &rest[..end];
        if url.to_lowercase().contains(&needle_lower) {
            return Some(url.to_string());
        }
        idx = start + 8; // past "https://"
        if idx >= html.len() {
            break;
        }
    }
    None
}

// ---------------------------------------------------------------------------
// schema.org Product JSON-LD
// ---------------------------------------------------------------------------

/// Extracts every `<script type="application/ld+json">` block on the page
/// that describes (or contains, via `@graph`) a schema.org `Product`.
/// Structured data like this is what search engines rely on for rich
/// results, so storefronts keep it accurate — it's a much sturdier source
/// than reverse-engineering CSS class names.
fn ld_json_products(html: &str) -> Vec<Json> {
    let mut out = Vec::new();
    let marker = r#"type="application/ld+json""#;
    let marker2 = r#"type='application/ld+json'"#;
    let mut idx = 0usize;

    while idx < html.len() {
        let Some(rel) = find_ci(&html[idx..], marker).or_else(|| find_ci(&html[idx..], marker2)) else { break };
        let tag_open_start = idx + rel;
        let Some(gt_rel) = html[tag_open_start..].find('>') else { break };
        let content_start = tag_open_start + gt_rel + 1;
        let Some(close_rel) = find_ci(&html[content_start..], "</script>") else { break };
        let content_end = content_start + close_rel;
        let json_text = &html[content_start..content_end];

        if let Ok(value) = serde_json::from_str::<Json>(json_text) {
            collect_products(&value, &mut out);
        }

        idx = content_end + "</script>".len();
    }
    out
}

fn collect_products(value: &Json, out: &mut Vec<Json>) {
    match value {
        Json::Array(items) => {
            for item in items {
                collect_products(item, out);
            }
        }
        Json::Object(map) => {
            let is_product = match map.get("@type") {
                Some(Json::String(s)) => s.eq_ignore_ascii_case("product"),
                Some(Json::Array(types)) => types
                    .iter()
                    .any(|t| t.as_str().map(|s| s.eq_ignore_ascii_case("product")).unwrap_or(false)),
                _ => false,
            };
            if is_product {
                out.push(value.clone());
            }
            if let Some(graph) = map.get("@graph") {
                collect_products(graph, out);
            }
        }
        _ => {}
    }
}

/// The first Product JSON-LD block on the page, if any.
fn ld_json_product(html: &str) -> Option<Json> {
    ld_json_products(html).into_iter().next()
}

fn ld_str(value: &Json, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// `brand` can be a plain string or `{ "name": "...", "url": "..." }`.
fn ld_brand_name(product: &Json) -> Option<String> {
    match product.get("brand") {
        Some(Json::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Some(brand @ Json::Object(_)) => ld_str(brand, "name"),
        _ => None,
    }
    .or_else(|| {
        // Some storefronts describe the shop as "seller" or "manufacturer" instead.
        product
            .get("seller")
            .and_then(|s| ld_str(s, "name"))
            .or_else(|| product.get("manufacturer").and_then(|s| ld_str(s, "name")))
    })
}

/// `image` can be a single URL string, an array of URL strings, or an array
/// of `ImageObject`s with a `url` field.
fn ld_images(product: &Json) -> Vec<String> {
    let mut out = Vec::new();
    match product.get("image") {
        Some(Json::String(s)) if !s.trim().is_empty() => out.push(s.trim().to_string()),
        Some(Json::Array(items)) => {
            for item in items {
                match item {
                    Json::String(s) if !s.trim().is_empty() => out.push(s.trim().to_string()),
                    Json::Object(_) => {
                        if let Some(u) = ld_str(item, "url") {
                            out.push(u);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    out
}

// ---------------------------------------------------------------------------
// Generic image collection (JSON-LD + og:image + gallery heuristic)
// ---------------------------------------------------------------------------

/// Generic gallery/carousel scan: walks every `<img>` on the page and keeps
/// ones that look like product photos rather than chrome (logos, icons,
/// avatars, spinners...). An image is kept if it either sits near markup
/// that looks gallery-related, or its URL matches one of the site's known
/// image-CDN hints.
fn find_gallery_images(html: &str, domain_hints: &[&str]) -> Vec<String> {
    const GALLERY_HINTS: [&str; 10] = [
        "gallery", "carousel", "slider", "swiper", "thumbnail",
        "product-image", "item-image", "photos", "slides", "lightbox",
    ];
    const EXCLUDE_HINTS: [&str; 10] = [
        "logo", "icon", "avatar", "sprite", "favicon", "badge", "flag",
        "spinner", "placeholder", "loading",
    ];
    const IMG_EXTS: [&str; 5] = [".jpg", ".jpeg", ".png", ".gif", ".webp"];
    const SRC_ATTRS: [&str; 6] = [
        "data-origin", "data-original", "data-full", "data-src", "data-lazy-src", "src",
    ];

    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (tag_start, tag_end) in find_tags(html, "<img") {
        let tag = &html[tag_start..tag_end];
        let tag_lower = tag.to_lowercase();

        // Look back at most 600 bytes for gallery-ish markup, snapped to a
        // real character boundary — `html` here is often Japanese product
        // copy, so a fixed byte offset can otherwise land mid-character.
        let window_start = floor_char_boundary(html, tag_start.saturating_sub(600));
        let window = html[window_start..tag_start].to_lowercase();
        let looks_gallery = GALLERY_HINTS.iter().any(|h| window.contains(h) || tag_lower.contains(h));

        for attr in SRC_ATTRS {
            if let Some(v) = extract_attr(tag, attr) {
                let v = v.trim().to_string();
                let v_lower = v.to_lowercase();
                if !v_lower.starts_with("http") {
                    continue;
                }
                let has_ext = IMG_EXTS.iter().any(|e| v_lower.contains(e));
                if !has_ext {
                    continue;
                }
                let excluded = EXCLUDE_HINTS.iter().any(|e| v_lower.contains(e));
                if excluded {
                    continue;
                }
                let domain_ok = domain_hints.iter().any(|d| v_lower.contains(d));
                if (looks_gallery || domain_ok) && seen.insert(v.clone()) {
                    out.push(v);
                }
                break; // one src-like attribute per <img> is enough
            }
        }

        if out.len() >= MAX_IMAGES {
            break;
        }
    }

    out
}

/// Normalizes an image URL to a dedup key so the same photo served at two
/// different CDN resize variants collapses to one entry. Strips the query
/// string, any `/c/<dims>/`-style resize path segment (Booth's convention;
/// harmless no-op on URLs that don't have one), and common resize suffixes
/// right before the extension.
fn image_dedupe_key(url: &str) -> String {
    let mut u = url.split('?').next().unwrap_or(url).to_lowercase();

    if let Some(pos) = u.find("/c/") {
        if let Some(rel) = u[pos + 3..].find('/') {
            let seg_end = pos + 3 + rel + 1;
            u = format!("{}/{}", &u[..pos], &u[seg_end..]);
        }
    }

    for suffix in ["_base_resized", "_resized", "_thumbnail", "_thumb", "_small", "_medium", "_large", "_grid"] {
        u = u.replace(suffix, "");
    }

    u
}

/// Higher is "better" (more likely full-resolution). Used to pick which of
/// two same-key URLs to keep.
fn image_rank(url: &str) -> i64 {
    let lower = url.to_lowercase();
    if let Some(pos) = lower.find("/c/") {
        let seg = &lower[pos + 3..];
        let seg = &seg[..seg.find('/').unwrap_or(seg.len())];
        let dims = seg.split(['x', '_']).next().unwrap_or("");
        dims.parse::<i64>().unwrap_or(0)
    } else {
        // No resize-path prefix at all usually means the original/full image.
        i64::MAX
    }
}

/// Adds a URL to `out`/`seen`, replacing the stored entry for the same
/// dedup key if the new one ranks higher (i.e. looks like a larger image).
fn push_image(out: &mut Vec<String>, index_of: &mut std::collections::HashMap<String, usize>, url: String) {
    let key = image_dedupe_key(&url);
    let rank = image_rank(&url);
    if let Some(&i) = index_of.get(&key) {
        if rank > image_rank(&out[i]) {
            out[i] = url;
        }
    } else {
        index_of.insert(key, out.len());
        out.push(url);
    }
}

/// Combines JSON-LD `image`, every `og:image` tag, and a generic gallery
/// scan, deduplicated (by normalized key, not just exact string) and capped.
fn collect_images(html: &str, domain_hints: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut index_of = std::collections::HashMap::new();

    if let Some(product) = ld_json_product(html) {
        for u in ld_images(&product) {
            push_image(&mut out, &mut index_of, u);
        }
    }
    for u in meta_all(html, "property", "og:image") {
        push_image(&mut out, &mut index_of, u);
    }
    for u in find_gallery_images(html, domain_hints) {
        push_image(&mut out, &mut index_of, u);
    }

    out.truncate(MAX_IMAGES);
    out
}

/// Best-effort search for a single "creator/shop icon"-style image near any
/// of the given class-name hints. Returns None rather than guessing wrong —
/// a missing icon just means the new creator gets no icon, same as before.
fn find_icon_near(html: &str, hints: &[&str]) -> Option<String> {
    for (tag_start, tag_end) in find_tags(html, "<img") {
        let tag = &html[tag_start..tag_end];
        let tag_lower = tag.to_lowercase();
        let window_start = floor_char_boundary(html, tag_start.saturating_sub(300));
        let window = html[window_start..tag_start].to_lowercase();
        let matches_hint = hints.iter().any(|h| tag_lower.contains(h) || window.contains(h));
        if !matches_hint {
            continue;
        }
        for attr in ["data-origin", "data-original", "data-src", "src"] {
            if let Some(v) = extract_attr(tag, attr) {
                let v = v.trim().to_string();
                if v.to_lowercase().starts_with("http") {
                    return Some(v);
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Generic Open Graph fallback
// ---------------------------------------------------------------------------

fn scrape_og(html: &str, site: &'static str) -> ScrapedMeta {
    let product = ld_json_product(html);
    ScrapedMeta {
        name: product.as_ref().and_then(|p| ld_str(p, "name"))
            .or_else(|| meta(html, "property", "og:title"))
            .or_else(|| og_title_from_tag(html)),
        creator_name: product.as_ref().and_then(ld_brand_name)
            .or_else(|| meta(html, "name", "author")),
        creator_icon_url: None,
        creator_profile_url: None,
        image_urls: collect_images(html, &[]),
        site_name: Some(site.to_string()),
    }
}

fn og_title_from_tag(html: &str) -> Option<String> {
    let s = find_ci(html, "<title")?;
    let rest = &html[s..];
    let content_start = rest.find('>')?;
    let rest = &rest[content_start + 1..];
    let end = rest.find("</title")?;
    let title = strip_tags(&rest[..end]);
    if title.is_empty() { None } else { Some(title) }
}

// ---------------------------------------------------------------------------
// Booth (booth.pm)
// ---------------------------------------------------------------------------

/// Booth-specific gallery scan matching its verified markup: every gallery
/// photo is an `<img class="market-item-detail-item-image" ...>` (the
/// 72x72 filmstrip thumbnails alongside it carry no such class, so they're
/// naturally excluded). `data-origin` holds the full-resolution original;
/// `data-lazy` and `src` are resized fallbacks for images that haven't been
/// lazy-loaded yet. Using this instead of a generic scan avoids the
/// duplicate-at-different-resolutions problem entirely, since each gallery
/// photo appears in exactly one `<img>` here.
fn booth_gallery_images(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (start, end) in find_tags(html, "<img") {
        let tag = &html[start..end];
        if !tag.to_lowercase().contains("market-item-detail-item-image") {
            continue;
        }
        let url = extract_attr(tag, "data-origin")
            .or_else(|| extract_attr(tag, "data-lazy"))
            .or_else(|| extract_attr(tag, "src"));
        if let Some(u) = url {
            let u = u.trim().to_string();
            if u.to_lowercase().starts_with("http") && seen.insert(u.clone()) {
                out.push(u);
            }
        }
        if out.len() >= MAX_IMAGES {
            break;
        }
    }
    out
}

/// Reads the creator's shop URL out of Booth's embedded page data, e.g.
/// `"url":"https://komado.booth.pm/"` inside a JSON blob the page ships
/// for its own hydration/analytics. This is present on every item page —
/// including ones fetched via the shared `booth.pm/<locale>/items/<id>`
/// form — regardless of whether the nav's home-link renders the way
/// `scrape_booth`'s other lookups expect, so it's tried first.
fn booth_json_shop_url(html: &str) -> Option<String> {
    let needle = r#""url":"https://"#;
    let mut idx = 0usize;
    while let Some(rel) = find_ci(&html[idx..], needle) {
        let value_start = idx + rel + needle.len() - "https://".len();
        let rest = &html[value_start..];
        let end = rest.find('"').unwrap_or(rest.len());
        let url = &rest[..end];
        if find_ci(url, ".booth.pm").is_some() {
            return Some(url.trim_end_matches('/').to_string());
        }
        idx = value_start + 1;
        if idx >= html.len() {
            break;
        }
    }
    None
}

/// Reads the creator's account handle out of a `data-subdomain="komado"`
/// attribute in the page's embedded data — the same handle the nickname
/// link is meant to expose, but from a plain attribute rather than nested
/// nav markup, so it survives nav layout changes the nickname-link lookup
/// doesn't.
fn booth_data_subdomain(html: &str) -> Option<String> {
    let value = extract_attr(html, "data-subdomain")?;
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn scrape_booth(html: &str) -> ScrapedMeta {
    let product = ld_json_product(html);

    let name = product.as_ref().and_then(|p| ld_str(p, "name"))
        .or_else(|| {
            meta(html, "property", "og:title").map(|t| {
                // Booth appends " - BOOTH" to titles; strip it.
                t.trim_end_matches(" - BOOTH")
                    .trim_end_matches(" | BOOTH")
                    .trim()
                    .to_string()
            })
        })
        .filter(|s| !s.is_empty());

    // Prefer Booth's own gallery markup (verified against real item pages);
    // fall back to the generic OG/gallery scan only if that comes up empty
    // (e.g. Booth changes its markup in the future).
    let mut image_urls = booth_gallery_images(html);
    if image_urls.is_empty() {
        image_urls = collect_images(html, &["booth.pximg.net"]);
    }

    // Creator: Booth pages carry two different *kinds* of name, and this
    // order matters. The account handle (e.g. "komado") is the stable
    // identifier — what a creator is actually called/searched for, and what
    // this app's own "Edit creator" name should match — and is what we
    // want. It's exposed twice: a `data-subdomain="komado"` attribute in
    // the page's embedded data (tried first — a plain attribute, so it
    // survives nav markup changes) and, redundantly, the nav's home-link
    // nickname text (tried next, for older pages without the attribute).
    // JSON-LD's "brand"/the shop's display name (from `<h1 class="booth-title">`)
    // is often a decorated marketing name (e.g. "lilLab" vs. the handle
    // "lil") rather than the identifier we want, so it's a fallback, along
    // with an author meta tag and the older shop-name link some page
    // templates still use.
    let nickname_link = between(html, r#"class="home-link-container__nickname"#, "</a>");
    let creator_name = booth_data_subdomain(html)
        .or_else(|| {
            nickname_link
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        })
        .or_else(|| product.as_ref().and_then(ld_brand_name))
        .or_else(|| meta(html, "name", "author"))
        .or_else(|| {
            between(html, r#"class="shop-name"#, "</a>")
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        });

    // Profile URL: prefer the embedded `"url":"https://komado.booth.pm/"`
    // JSON field — it's present regardless of which URL form the item was
    // fetched through. Fall back to the nav home-link's `href`, but only
    // when it's absolute: on a shop's own subdomain
    // (lilxyzw.booth.pm/items/...) Booth renders it as a relative "/",
    // which would resolve to nothing useful here — that's fine, since
    // `scrape()`'s fallback derives the right origin from the fetched URL
    // in that case.
    let creator_profile_url = booth_json_shop_url(html)
        .or_else(|| {
            nickname_link
                .and_then(|s| extract_attr(s, "href"))
                .filter(|href| href.starts_with("http://") || href.starts_with("https://"))
                .and_then(|href| origin_of(&href))
        });

    // Booth shop icons are always served from a `.../users/<id>/icon_image/...`
    // path — matching on that distinctive URL segment survives markup
    // changes (and CSS-background-image icons) better than any class name.
    let creator_icon_url = find_url_containing(html, "icon_image/")
        .or_else(|| find_icon_near(html, &["shop-icon", "shop__icon", "shop-thumbnail", "avatar"]));

    ScrapedMeta {
        name,
        creator_name,
        creator_icon_url,
        creator_profile_url,
        image_urls,
        site_name: Some("Booth".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Gumroad (gumroad.com)
// ---------------------------------------------------------------------------

/// Gumroad renders product pages with Inertia.js: essentially the entire
/// page's data — full gallery, seller name/avatar/profile URL, price,
/// description, etc. — ships as one HTML-entity-encoded JSON blob in
/// `<div id="app" data-page="...">`, and the surrounding markup is a
/// near-empty client-rendering shell. That's *why* scanning for `<img>`
/// tags (what `find_gallery_images`/`find_icon_near` do) only ever turns up
/// whatever's in `<meta>` tags: there's usually no other image markup in
/// the HTML Gumroad actually sends. This blob is the real source of truth
/// for this site, so it's tried first; the OG/JSON-LD/gallery-scan path
/// below remains as a fallback for pages where it's missing (e.g. if
/// Gumroad changes this format in the future).
fn gumroad_page_json(html: &str) -> Option<Json> {
    let raw = extract_attr(html, "data-page")?;
    serde_json::from_str(&decode_entities(&raw)).ok()
}

/// Extracts `product.covers` image URLs in display order, preferring each
/// cover's full-resolution `original_url` over its resized `url`. A cover
/// can also be a `"type":"oembed"` (Gumroad allows embedding a YouTube
/// video as one of the gallery slots) — those are skipped since they're
/// not a fetchable product photo.
fn gumroad_cover_images(product: &Json) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(Json::Array(covers)) = product.get("covers") {
        for cover in covers {
            if cover.get("type").and_then(|t| t.as_str()) != Some("image") {
                continue;
            }
            if let Some(url) = ld_str(cover, "original_url").or_else(|| ld_str(cover, "url")) {
                out.push(url);
            }
            if out.len() >= MAX_IMAGES {
                break;
            }
        }
    }
    out
}

/// `seller.profile_url` from the page JSON often carries a tracking query
/// string (e.g. `?recommended_by=search`) — strip it down to a clean shop
/// link, the same way Booth's `creator_profile_url` is normalized.
fn gumroad_seller_profile_url(seller: &Json) -> Option<String> {
    let raw = ld_str(seller, "profile_url")?;
    let clean = raw.split('?').next().unwrap_or(&raw).trim_end_matches('/');
    if clean.is_empty() { None } else { Some(clean.to_string()) }
}

fn scrape_gumroad(html: &str) -> ScrapedMeta {
    let page_json = gumroad_page_json(html);
    let product = page_json.as_ref().and_then(|p| p.get("props")).and_then(|p| p.get("product"));
    let seller = product.and_then(|p| p.get("seller"));

    let ld_product = ld_json_product(html);

    // Gumroad OG tags are reliable.
    let og_name = meta(html, "property", "og:title").filter(|s| !s.is_empty());
    let name_raw = product.and_then(|p| ld_str(p, "name"))
        .or_else(|| ld_product.as_ref().and_then(|p| ld_str(p, "name")))
        .or_else(|| og_name.clone());

    // The page-JSON gallery (see `gumroad_page_json`'s doc comment) is the
    // only source that reliably has every image; fall back to the old
    // JSON-LD/OG/gallery-scan path if the page JSON wasn't found or had no
    // image covers.
    let mut image_urls = product.map(gumroad_cover_images).unwrap_or_default();
    if image_urls.is_empty() {
        image_urls = collect_images(html, &["public-files.gumroad.com"]);
    }

    // Creator: the page JSON's `seller.name` is Gumroad's own record of who
    // this is — prefer it. Otherwise fall back to JSON-LD brand/seller, an
    // author meta tag, the "Product Name by Creator Name" title pattern, or
    // og:site_name as a last resort (it's sometimes just "Gumroad").
    let creator_name = seller.and_then(|s| ld_str(s, "name"))
        .or_else(|| ld_product.as_ref().and_then(ld_brand_name))
        .or_else(|| meta(html, "name", "author"))
        .or_else(|| {
            name_raw.as_deref().and_then(|t| {
                rfind_ci(t, " by ").map(|pos| t[pos + 4..].trim().to_string())
            })
        })
        .or_else(|| meta(html, "property", "og:site_name"))
        .filter(|s| !s.is_empty() && s != "Gumroad");

    // Clean up " by Creator" from the name if we used that extraction.
    let name = name_raw.map(|t| match rfind_ci(&t, " by ") {
        Some(pos) => t[..pos].trim().to_string(),
        None => t,
    });

    // The page JSON's `seller.avatar_url` is a direct hit; the old
    // class-name scan is kept as a fallback for pages without it.
    let creator_icon_url = seller.and_then(|s| ld_str(s, "avatar_url"))
        .or_else(|| find_icon_near(html, &["seller-avatar", "creator-avatar", "profile-picture", "avatar"]));

    // Not previously populated for Gumroad at all (it relied entirely on
    // `scrape()`'s generic origin-of-the-fetched-URL fallback, which only
    // happens to work when the item URL is on the creator's own
    // subdomain). `seller.profile_url` is explicit and always correct.
    let creator_profile_url = seller.and_then(gumroad_seller_profile_url);

    ScrapedMeta {
        name,
        creator_name,
        creator_icon_url,
        creator_profile_url,
        image_urls,
        site_name: Some("Gumroad".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Jinxxy (jinxxy.com)
// ---------------------------------------------------------------------------

fn scrape_jinxxy(html: &str) -> ScrapedMeta {
    let product = ld_json_product(html);

    let name = product.as_ref().and_then(|p| ld_str(p, "name"))
        .or_else(|| meta(html, "property", "og:title"))
        .or_else(|| og_title_from_tag(html))
        .filter(|s| !s.is_empty());

    let image_urls = collect_images(html, &[]);

    // Jinxxy includes creator in the description like "by CreatorName"
    // or in a dedicated element.
    let creator_name = product.as_ref().and_then(ld_brand_name)
        .or_else(|| meta(html, "name", "author"))
        .or_else(|| {
            between(html, r#"class="creator-name"#, "</")
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            between(html, r#"class="username"#, "</")
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        })
        .filter(|s| !s.is_empty());

    let creator_icon_url = find_icon_near(html, &["creator-avatar", "creator-icon", "username", "avatar"]);

    ScrapedMeta {
        name,
        creator_name,
        creator_icon_url,
        creator_profile_url: None,
        image_urls,
        site_name: Some("Jinxxy".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Payhip (payhip.com)
// ---------------------------------------------------------------------------

fn scrape_payhip(html: &str) -> ScrapedMeta {
    let product = ld_json_product(html);

    let name = product.as_ref().and_then(|p| ld_str(p, "name"))
        .or_else(|| meta(html, "property", "og:title"))
        .or_else(|| og_title_from_tag(html))
        .map(|t| {
            // Payhip appends "| Payhip" or "- Payhip" to titles.
            t.trim_end_matches("| Payhip")
                .trim_end_matches("- Payhip")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty());

    let image_urls = collect_images(html, &["cdn.payhip.com"]);

    let creator_name = product.as_ref().and_then(ld_brand_name)
        .or_else(|| meta(html, "name", "author"))
        .or_else(|| {
            between(html, r#"class="seller-name"#, "</")
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            between(html, r#"class="store-name"#, "</")
                .and_then(|s| s.find('>').map(|i| &s[i + 1..]))
                .map(strip_tags)
                .filter(|s| !s.is_empty())
        })
        .filter(|s| !s.is_empty());

    let creator_icon_url = find_icon_near(html, &["seller-avatar", "seller-logo", "store-logo", "avatar"]);

    ScrapedMeta {
        name,
        creator_name,
        creator_icon_url,
        creator_profile_url: None,
        image_urls,
        site_name: Some("Payhip".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Returns the site display name if this URL is supported, null otherwise.
/// Cheap — no network call. Used by the frontend to decide whether to show
/// the "Fetch info" button.
#[tauri::command]
pub fn detect_supported_site(url: String) -> Option<String> {
    detect_site(&url).map(|s| s.to_string())
}

/// Fetches and parses metadata from a supported product page.
#[tauri::command]
pub async fn fetch_page_meta(url: String) -> Result<ScrapedMeta, String> {
    scrape(&url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page dense with multi-byte Japanese text (typical of a real Booth
    /// listing) immediately before an `<img>` tag must not panic when we
    /// look back through it for gallery/icon hint words.
    #[test]
    fn gallery_scan_survives_multibyte_lookback_window() {
        let filler = "商品説明とクリエイターについての長い日本語のテキストです。".repeat(20);
        let html = format!(
            r#"<div>{filler}<img class="market-item-detail-item-image" data-origin="https://booth.pximg.net/a.jpg"></div>"#
        );
        let images = find_gallery_images(&html, &["booth.pximg.net"]);
        assert_eq!(images, vec!["https://booth.pximg.net/a.jpg".to_string()]);

        let icon = find_icon_near(&html, &["market-item-detail-item-image"]);
        assert_eq!(icon.as_deref(), Some("https://booth.pximg.net/a.jpg"));
    }

    /// Unicode input where lowercasing changes a character's byte length
    /// (Turkish `İ`, U+0130, lowercases to two code points) must not
    /// desync `find_ci`/`find_tags` offsets from the original string.
    #[test]
    fn case_insensitive_search_survives_expanding_lowercase() {
        let html = r#"<div>İstanbul İstanbul İstanbul</div><META property="og:title" content="Hello">"#;
        assert_eq!(meta(html, "property", "og:title").as_deref(), Some("Hello"));
    }

    #[test]
    fn booth_nickname_is_read_from_current_home_link_markup() {
        let html = r#"
            <nav role="navigation">
              <div class="shop-global-nav__home-link">
                <div class="home-link-container">
                  <div class="home-link-container__nickname">
                    <a class="nav" title="Home" href="/">lil</a>
                  </div>
                </div>
              </div>
            </nav>
        "#;
        assert_eq!(scrape_booth(html).creator_name.as_deref(), Some("lil"));
    }

    /// Booth pages have two different names for the same shop: the account
    /// nickname ("lil") and a decorated marketing display name ("lilLab")
    /// that JSON-LD's `brand` and the `<h1 class="booth-title">` both use.
    /// The nickname is the one that should win.
    #[test]
    fn booth_nickname_wins_over_json_ld_brand_display_name() {
        let html = r#"
            <script type="application/ld+json">
            {"@type":"Product","name":"lilToon","brand":{"@type":"Brand","name":"lilLab"}}
            </script>
            <h1 class="booth-title has-image">lilLab</h1>
            <nav role="navigation">
              <div class="shop-global-nav__home-link">
                <div class="home-link-container">
                  <div class="home-link-container__nickname">
                    <a class="nav" title="Home" href="/">lil</a>
                  </div>
                </div>
              </div>
            </nav>
        "#;
        let scraped = scrape_booth(html);
        assert_eq!(scraped.creator_name.as_deref(), Some("lil"));
        assert_eq!(scraped.name.as_deref(), Some("lilToon"));
    }

    /// Item viewed via Booth's shared, non-subdomain URL
    /// (`booth.pm/<locale>/items/<id>` — the form people most commonly
    /// paste/share): the home-link's `href` must be absolute, since a
    /// relative "/" would point at booth.pm itself rather than the shop.
    /// `scrape_booth` should read the real shop subdomain straight off
    /// that absolute href rather than guessing from the fetched URL.
    #[test]
    fn booth_profile_url_uses_absolute_home_link_when_present() {
        let html = r#"
            <div class="home-link-container__nickname">
              <a class="nav" title="Home" href="https://komado.booth.pm/">あまとうさぎ</a>
            </div>
        "#;
        let scraped = scrape_booth(html);
        assert_eq!(scraped.creator_name.as_deref(), Some("あまとうさぎ"));
        assert_eq!(scraped.creator_profile_url.as_deref(), Some("https://komado.booth.pm"));
    }

    /// Item viewed via the shop's own subdomain: Booth renders the
    /// home-link as relative, since we're already on that subdomain.
    /// `scrape_booth` alone has no base URL to resolve a relative href
    /// against, so it must leave `creator_profile_url` unset and let
    /// `scrape()`'s fallback (the origin of the fetched URL, which is
    /// already correct in this case) fill it in.
    #[test]
    fn booth_profile_url_left_unset_for_relative_home_link() {
        let html = r#"
            <div class="home-link-container__nickname">
              <a class="nav" title="Home" href="/">lil</a>
            </div>
        "#;
        let scraped = scrape_booth(html);
        assert_eq!(scraped.creator_name.as_deref(), Some("lil"));
        assert!(scraped.creator_profile_url.is_none());
    }

    /// When the page carries Booth's embedded `data-subdomain` attribute
    /// and `"url":"https://...booth.pm/"` JSON field, both should win over
    /// the nav home-link — even when the home-link is present and would
    /// otherwise give a different (decorated) name.
    #[test]
    fn booth_prefers_data_subdomain_and_json_url_over_home_link() {
        let html = r#"
            <div data-subdomain="komado" data-other="ignore-me"></div>
            <script>window.__DATA__ = {"shop":{"url":"https://komado.booth.pm/","id":1}};</script>
            <div class="home-link-container__nickname">
              <a class="nav" title="Home" href="https://lil.booth.pm/">lil</a>
            </div>
        "#;
        let scraped = scrape_booth(html);
        assert_eq!(scraped.creator_name.as_deref(), Some("komado"));
        assert_eq!(scraped.creator_profile_url.as_deref(), Some("https://komado.booth.pm"));
    }

    /// Pages without `data-subdomain` or an embedded shop `"url"` field
    /// (e.g. older markup) must still fall back to the nav home-link, same
    /// as before this fallback chain was extended.
    #[test]
    fn booth_falls_back_to_home_link_when_new_fields_absent() {
        let html = r#"
            <div class="home-link-container__nickname">
              <a class="nav" title="Home" href="https://komado.booth.pm/">komado</a>
            </div>
        "#;
        let scraped = scrape_booth(html);
        assert_eq!(scraped.creator_name.as_deref(), Some("komado"));
        assert_eq!(scraped.creator_profile_url.as_deref(), Some("https://komado.booth.pm"));
    }

    /// The `"url":"..."` JSON scan must ignore unrelated `"url"` fields
    /// (e.g. image URLs in the same blob) and only match ones that point
    /// at a `.booth.pm` shop domain.
    #[test]
    fn booth_json_shop_url_ignores_non_booth_url_fields() {
        let html = r#"<script>{"image":{"url":"https://booth.pximg.net/photo.jpg"},"shop":{"url":"https://komado.booth.pm/"}}</script>"#;
        assert_eq!(booth_json_shop_url(html).as_deref(), Some("https://komado.booth.pm"));
    }

    /// Real Gumroad product pages ship their data as an Inertia `data-page`
    /// JSON blob (see `gumroad_page_json`'s doc comment) rather than in
    /// `<img>` markup — this fixture mirrors that real shape (trimmed to
    /// the fields we read) to verify name/creator/icon/images/profile-url
    /// all come from it.
    #[test]
    fn gumroad_reads_name_creator_and_all_covers_from_inertia_page_json() {
        let page_json = r#"{"props":{"product":{"name":"Ari [FT | PC]","covers":[{"url":"https://public-files.gumroad.com/small1","original_url":"https://public-files.gumroad.com/full1","type":"image"},{"url":"https://www.youtube.com/embed/xyz","type":"oembed"},{"url":"https://public-files.gumroad.com/small2","original_url":"https://public-files.gumroad.com/full2","type":"image"}],"seller":{"name":"HoloExe","avatar_url":"https://public-files.gumroad.com/avatar1","profile_url":"https://holoexe.gumroad.com/?recommended_by=search"}}}}"#;
        let escaped = page_json.replace('"', "&quot;");
        let html = format!(r#"<div id="app" data-page="{escaped}" data-page-version="1"></div>"#);

        let scraped = scrape_gumroad(&html);
        assert_eq!(scraped.name.as_deref(), Some("Ari [FT | PC]"));
        assert_eq!(scraped.creator_name.as_deref(), Some("HoloExe"));
        assert_eq!(scraped.creator_icon_url.as_deref(), Some("https://public-files.gumroad.com/avatar1"));
        assert_eq!(scraped.creator_profile_url.as_deref(), Some("https://holoexe.gumroad.com"));
        // Both image covers, in order, using the full-resolution original —
        // and the oembed (video) cover excluded.
        assert_eq!(
            scraped.image_urls,
            vec![
                "https://public-files.gumroad.com/full1".to_string(),
                "https://public-files.gumroad.com/full2".to_string(),
            ]
        );
    }

    /// Pages without the Inertia `data-page` blob (or where it's missing
    /// `product`) must still fall back to the old JSON-LD/OG/gallery-scan
    /// path, same as before this fix.
    #[test]
    fn gumroad_falls_back_to_json_ld_and_og_when_page_json_absent() {
        let html = r#"
            <meta property="og:title" content="Some Asset by SomeCreator">
            <script type="application/ld+json">
            {"@type":"Product","name":"Some Asset","image":"https://public-files.gumroad.com/only.jpg"}
            </script>
        "#;
        let scraped = scrape_gumroad(html);
        assert_eq!(scraped.name.as_deref(), Some("Some Asset"));
        assert_eq!(scraped.creator_name.as_deref(), Some("SomeCreator"));
        assert_eq!(scraped.image_urls, vec!["https://public-files.gumroad.com/only.jpg".to_string()]);
        assert!(scraped.creator_icon_url.is_none());
        assert!(scraped.creator_profile_url.is_none());
    }

    #[test]
    fn origin_of_handles_malformed_and_edge_case_urls() {
        assert_eq!(origin_of("https://lilxyzw.booth.pm/items/123"), Some("https://lilxyzw.booth.pm".to_string()));
        assert_eq!(origin_of("https://lilxyzw.booth.pm"), Some("https://lilxyzw.booth.pm".to_string()));
        assert_eq!(origin_of("not-a-url"), None);
        assert_eq!(origin_of("https:///items/123"), None);
    }

    #[test]
    fn find_ci_handles_empty_and_oversized_needles() {
        assert_eq!(find_ci("hello", ""), Some(0));
        assert_eq!(find_ci("hi", "hello"), None);
        assert_eq!(find_ci("Hello World", "WORLD"), Some(6));
        assert_eq!(rfind_ci("by by by", "BY"), Some(6));
    }

    #[test]
    fn gumroad_by_split_survives_unicode_prefix() {
        // "İ" lowercases to two code points, expanding byte length before
        // the ASCII " by " separator — this must not desync the split.
        let name_raw = "İİİ Product by Creator Name".to_string();
        let pos = rfind_ci(&name_raw, " by ").unwrap();
        assert_eq!(&name_raw[pos + 4..], "Creator Name");
        assert_eq!(&name_raw[..pos], "İİİ Product");
    }
}
