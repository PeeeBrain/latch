use gpui_kit::RenderImage;
use image::{ImageFormat, ImageReader};
use std::{io::Cursor, net::IpAddr, sync::Arc, time::Duration};
use tokio::sync::watch;
use url::Url;

// Small icons should never monopolize a worker, connection, or decoder.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_BYTES: usize = 256 * 1024;
const MAX_DIMENSION: u32 = 512;
pub const MAX_CONCURRENT: usize = 4;
// At 48px per icon, this keeps decoded session data below 10 MiB.
pub const MAX_CACHED: usize = 1024;

/// Ignore saved icon URLs. A credential's website is the only source of its icon.
pub fn origin(website: Option<&str>) -> Option<Url> {
    let website = website?.trim();
    if website.is_empty() {
        return None;
    }
    let mut url = Url::parse(website)
        .or_else(|_| Url::parse(&format!("https://{website}")))
        .ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !valid_host(&url)
    {
        return None;
    }
    url.set_scheme("https").ok()?;
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Some(url)
}

fn valid_host(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(host)) => {
            let host = host.trim_end_matches('.');
            host.contains('.')
                && !["localhost", "local", "internal", "test", "invalid", "onion"]
                    .iter()
                    .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
        }
        // IP-based credentials keep their initial tile, including public IPs.
        _ => false,
    }
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && b == 0 && c == 0)
                && !(a == 192 && b == 88 && c == 99)
                && !(a == 198 && (18..=19).contains(&b))
        }
        IpAddr::V6(ip) => {
            let [a, b, ..] = ip.segments();
            // Only global unicast; exclude special, documentation, and 6to4 ranges.
            a & 0xe000 == 0x2000
                && !(a == 0x2001 && b < 0x0200)
                && !(a == 0x2001 && b == 0x0db8)
                && a != 0x2002
                && !(a == 0x3fff && b < 0x1000)
        }
    }
}

pub fn load(url: Url, mut cancel: watch::Receiver<()>) -> Option<Arc<RenderImage>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    let result = runtime.block_on(async {
        tokio::select! {
            biased;
            _ = cancel.changed() => None,
            result = tokio::time::timeout(REQUEST_TIMEOUT, download(url)) => result.ok().flatten(),
        }
    });
    // A canceled system DNS lookup may outlive its future. Do not wait for
    // Tokio's blocking resolver threads when releasing the GPUI worker.
    runtime.shutdown_background();
    result
}

async fn download(mut url: Url) -> Option<Arc<RenderImage>> {
    // shortcut: use the conventional icon path; add HTML discovery if sites
    // without /favicon.ico need coverage, rather than introducing a tracking proxy.
    url.set_path("/favicon.ico");
    for _ in 0..=3 {
        if url.scheme() != "https"
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || !valid_host(&url)
        {
            return None;
        }
        let host = url.host_str()?;
        let addresses: Vec<_> = tokio::net::lookup_host((host, 443)).await.ok()?.collect();
        if addresses.is_empty() || addresses.iter().any(|addr| !public_address(addr.ip())) {
            return None;
        }
        // Pin DNS results and bypass proxies so a checked name cannot resolve
        // to a private address later. Every redirect goes through this check.
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .referer(false)
            .timeout(REQUEST_TIMEOUT)
            .resolve_to_addrs(host, &addresses)
            .build()
            .ok()?;
        let mut response = client.get(url.clone()).send().await.ok()?;
        if response.status().is_redirection() {
            url = url
                .join(
                    response
                        .headers()
                        .get(reqwest::header::LOCATION)?
                        .to_str()
                        .ok()?,
                )
                .ok()?;
            continue;
        }
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|size| size > MAX_BYTES as u64)
        {
            return None;
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if bytes.len().checked_add(chunk.len())? > MAX_BYTES {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        return decode(&bytes);
    }
    None
}

fn decode(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    if !matches!(reader.format(), Some(ImageFormat::Ico | ImageFormat::Png)) {
        return None;
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(4 * 1024 * 1024);
    reader.limits(limits);
    let mut pixels = reader.decode().ok()?.thumbnail(48, 48).into_rgba8();
    // GPUI uploads BGRA rather than image-rs's RGBA.
    for pixel in pixels.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new([image::Frame::new(pixels)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_website_origins_become_icon_sources() {
        for website in [
            " example.com/sign-in?token=secret#fragment ",
            "http://EXAMPLE.com/account",
            "https://example.com:443/",
        ] {
            assert_eq!(
                origin(Some(website)).unwrap().as_str(),
                "https://example.com/"
            );
        }
        for website in [
            "",
            "localhost",
            "https://router.local/",
            "https://login.internal/",
            "https://localhost./",
            "https://example.invalid/",
            "https://127.0.0.1/",
            "https://2130706433/",
            "https://[::1]/",
            "file:///etc/passwd",
            "ftp://example.com/",
            "https://user:secret@example.com/",
            "https://example.com:8443/",
            "data:image/png;base64,abc",
        ] {
            assert!(origin(Some(website)).is_none(), "{website}");
        }
        assert!(origin(None).is_none());
    }

    #[test]
    fn dns_targets_must_be_public_unicast_addresses() {
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(public_address(address.parse().unwrap()), "{address}");
        }
        for address in [
            "0.0.0.0",
            "0.1.2.3",
            "10.1.2.3",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "2001::1",
            "2001:db8::1",
            "2002:7f00:1::",
            "3fff::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
        ] {
            assert!(!public_address(address.parse().unwrap()), "{address}");
        }
    }

    #[test]
    fn icons_decode_to_bgra_with_size_and_format_limits() {
        for format in [ImageFormat::Png, ImageFormat::Ico] {
            let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128]));
            let mut bytes = Cursor::new(Vec::new());
            pixels.write_to(&mut bytes, format).unwrap();
            let icon = decode(bytes.get_ref()).unwrap();
            assert_eq!(&icon.as_bytes(0).unwrap()[..4], &[0, 0, 255, 128]);
            assert_eq!(icon.frame_count(), 1);
        }
        let pixels = image::RgbaImage::new(MAX_DIMENSION + 1, 1);
        let mut bytes = Cursor::new(Vec::new());
        pixels.write_to(&mut bytes, ImageFormat::Png).unwrap();
        assert!(decode(bytes.get_ref()).is_none());
        assert!(decode(b"<html>not an icon</html>").is_none());
        assert!(decode(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_none());
    }

    #[test]
    #[ignore = "requires public internet; verifies direct icons and a cross-host HTTPS redirect"]
    fn live_website_icons() {
        let (_cancel, receiver) = watch::channel(());
        for website in ["https://github.com", "https://mail.google.com"] {
            assert!(
                load(origin(Some(website)).unwrap(), receiver.clone()).is_some(),
                "{website}"
            );
        }
    }
}
