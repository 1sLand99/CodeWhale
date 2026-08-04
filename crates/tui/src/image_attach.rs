//! Turning a local image file into a wire-ready image content block.
//!
//! CodeWhale's message model has been multimodal for a long time —
//! [`ContentBlock::ImageUrl`](crate::models::ContentBlock::ImageUrl) round-trips
//! through session persistence, compaction and all three wire builders. What it
//! never had was a *faucet*: nothing outside `#[cfg(test)]` ever constructed
//! one, so a user who attached a screenshot got the literal text
//! `[Attached image: /path/to/shot.png]` and a model that correctly concluded it
//! could not see the picture.
//!
//! This module is that faucet. It reads a file, proves it is an image the
//! providers actually accept, holds it to a size budget, and emits a
//! `data:` URL.
//!
//! # Two stages, because the two failure kinds differ
//!
//! [`expand_attachment_blocks`] runs when the message is built and decides the
//! *permanent* questions: does this file exist, is it really an image, is it
//! small enough. Those answers cannot change, so they are baked into history.
//!
//! [`strip_images_when_unsupported`] runs per outbound request and decides the
//! one *contingent* question: can the model this request is going to actually
//! see images. Routes change mid-session, so answering that at build time
//! would mean attaching a screenshot under a text-only model and losing it
//! permanently, even after switching to a vision model. History keeps the
//! image; each request is normalized against its own route.
//!
//! # Provider neutrality
//!
//! There is exactly one internal representation — a `data:<media-type>;base64,…`
//! URL on `ContentBlock::ImageUrl` — and each wire builder projects it:
//!
//! | wire format | shape |
//! |---|---|
//! | Chat Completions | `{"type":"image_url","image_url":{"url":"data:…"}}` |
//! | Responses | `{"type":"input_image","image_url":"data:…"}` |
//! | Anthropic Messages | `{"type":"image","source":{"type":"base64","media_type":…,"data":…}}` |
//!
//! The Anthropic split lives in [`parse_data_url`], which
//! `client::anthropic` calls. Anthropic is the reason the accepted-format list
//! below is not simply "whatever the `image` crate can decode": it accepts only
//! PNG/JPEG/GIF/WebP, and so, therefore, do we. Refusing a BMP here with a
//! readable message beats letting one through to a provider-side 400.

use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::model_profile::SupportState;
use crate::models::{ContentBlock, ImageUrlContent};

/// Largest source image accepted, in bytes, before base64 expansion.
///
/// Base64 inflates by 4/3, so this admits roughly 6.7 MB of request body per
/// image. The number is Anthropic's documented per-image ceiling; keeping the
/// tightest provider limit as the shared limit is what makes a "CodeWhale
/// accepted it" verdict portable across routes.
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Why a file could not be attached as an image.
///
/// Every variant renders to a sentence naming the file and the reason. These
/// strings reach both the user (as a command error) and the model (as an
/// in-band notice), so they say what to do next rather than only what failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageAttachError {
    /// The file could not be read at all.
    Unreadable { path: String, reason: String },
    /// The file is zero bytes.
    Empty { path: String },
    /// Over [`MAX_IMAGE_BYTES`].
    TooLarge { path: String, bytes: usize },
    /// Magic bytes identify a format no provider in the set accepts.
    UnsupportedFormat { path: String, detected: String },
    /// Magic bytes match nothing we recognize as an image.
    NotAnImage { path: String },
}

impl std::fmt::Display for ImageAttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path, reason } => {
                write!(f, "Cannot attach {path}: {reason}")
            }
            Self::Empty { path } => {
                write!(f, "Cannot attach {path}: the file is empty")
            }
            Self::TooLarge { path, bytes } => write!(
                f,
                "Cannot attach {path}: {} exceeds the {} per-image limit. \
                 Downscale or crop it first.",
                human_bytes(*bytes),
                human_bytes(MAX_IMAGE_BYTES),
            ),
            Self::UnsupportedFormat { path, detected } => write!(
                f,
                "Cannot attach {path}: {detected} is not accepted by vision \
                 models. Convert it to PNG, JPEG, GIF or WebP.",
            ),
            Self::NotAnImage { path } => write!(
                f,
                "Cannot attach {path}: the file is not a PNG, JPEG, GIF or \
                 WebP image (its contents do not match any of those formats).",
            ),
        }
    }
}

impl std::error::Error for ImageAttachError {}

fn human_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} bytes")
    }
}

/// An image that is ready to go on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedImage {
    /// e.g. `"image/png"`.
    pub media_type: &'static str,
    /// `data:<media_type>;base64,<payload>`.
    pub data_url: String,
    /// Size of the source file, before base64.
    pub source_bytes: usize,
}

impl AttachedImage {
    /// The content block this image becomes in a message.
    #[must_use]
    pub fn content_block(&self) -> ContentBlock {
        ContentBlock::ImageUrl {
            image_url: ImageUrlContent {
                url: self.data_url.clone(),
            },
        }
    }
}

/// Identify an image format from its leading bytes.
///
/// Extension sniffing is not enough here: the extension is attacker- and
/// typo-controlled, while what the provider validates is the payload. A
/// `.png` holding JPEG bytes must be declared `image/jpeg` or the request is
/// rejected with a media-type mismatch that reads like a CodeWhale bug.
///
/// Returns `None` for anything that is not one of the four accepted formats;
/// [`detect_rejected_format`] names the near-misses so the error can be
/// specific.
#[must_use]
pub fn sniff_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

/// Name a format we can recognize but deliberately refuse.
///
/// These are real images, so `NotAnImage` would be a lie and would send the
/// user looking for a corrupt file. Naming the format points at the fix
/// (convert it) instead.
#[must_use]
pub fn detect_rejected_format(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"BM") {
        return Some("BMP");
    }
    if bytes.starts_with(b"II\x2a\x00") || bytes.starts_with(b"MM\x00\x2a") {
        return Some("TIFF");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"\0\0\0") && &bytes[4..8] == b"ftyp" {
        return Some("HEIC/AVIF");
    }
    if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        return Some("SVG");
    }
    if bytes.starts_with(b"%PDF") {
        return Some("PDF");
    }
    None
}

/// Validate and encode raw bytes that were read from `path`.
///
/// Split from [`attach_image_from_path`] so the whole policy — order of
/// checks, limits, format verdicts — is testable without touching a
/// filesystem.
pub fn encode_image_bytes(bytes: &[u8], path: &str) -> Result<AttachedImage, ImageAttachError> {
    if bytes.is_empty() {
        return Err(ImageAttachError::Empty {
            path: path.to_string(),
        });
    }
    // Size is checked before format so a huge file is rejected on the cheap
    // fact rather than after we have decided what it is.
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageAttachError::TooLarge {
            path: path.to_string(),
            bytes: bytes.len(),
        });
    }
    let Some(media_type) = sniff_media_type(bytes) else {
        return Err(match detect_rejected_format(bytes) {
            Some(detected) => ImageAttachError::UnsupportedFormat {
                path: path.to_string(),
                detected: detected.to_string(),
            },
            None => ImageAttachError::NotAnImage {
                path: path.to_string(),
            },
        });
    };
    let payload = STANDARD.encode(bytes);
    Ok(AttachedImage {
        media_type,
        data_url: format!("data:{media_type};base64,{payload}"),
        source_bytes: bytes.len(),
    })
}

/// Read, validate and encode an image file.
pub fn attach_image_from_path(path: &Path) -> Result<AttachedImage, ImageAttachError> {
    let display = path.display().to_string();
    // Check the size from metadata first so a multi-gigabyte file is refused
    // without being read into memory.
    if let Ok(meta) = std::fs::metadata(path) {
        let len = meta.len();
        if len > MAX_IMAGE_BYTES as u64 {
            return Err(ImageAttachError::TooLarge {
                path: display,
                bytes: usize::try_from(len).unwrap_or(usize::MAX),
            });
        }
    }
    let bytes = std::fs::read(path).map_err(|error| ImageAttachError::Unreadable {
        path: display.clone(),
        reason: error.to_string(),
    })?;
    encode_image_bytes(&bytes, &display)
}

/// Split a `data:<media-type>;base64,<payload>` URL.
///
/// Anthropic's Messages API models an image as a tagged `source` rather than a
/// URL, so the native route has to take the data URL back apart. Returns
/// `None` for `http(s)` URLs and for anything malformed, which the caller
/// renders as a remote source or a visible degradation respectively.
#[must_use]
pub fn parse_data_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("data:")?;
    let (header, payload) = rest.split_once(',')?;
    let media_type = header.strip_suffix(";base64")?;
    if media_type.is_empty() || payload.is_empty() {
        return None;
    }
    Some((media_type, payload))
}

/// Whether a URL is one a provider can fetch for itself.
#[must_use]
pub fn is_remote_image_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// The outcome of expanding a user turn's attachment placeholders.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpandedAttachments {
    /// Image blocks to append to the user message, in placeholder order.
    pub blocks: Vec<ContentBlock>,
    /// One line per attachment that could not be sent. These are shown to the
    /// user and also handed to the model, because a model that is not told an
    /// image was dropped will confidently discuss it from the filename.
    pub notices: Vec<String>,
}

/// Build the image blocks for a user turn from its `[Attached image: …]` lines.
///
/// This is the ingest half of the composer's placeholder design: the buffer
/// holds a path-bearing text line (which survives editing, history and session
/// reload for free), and the bytes are read here, once, as the message is
/// built.
///
/// Only *permanent* failures are decided here — a file that is missing,
/// oversized, or not an image will still be all of those things next turn, so
/// baking the verdict into history costs nothing. Whether the *model* can see
/// images is deliberately not decided here; that is contingent on the active
/// route and is re-decided per request by
/// [`strip_images_when_unsupported`].
///
/// Each image is bracketed by text tags naming its path. Without them a turn
/// carrying three screenshots gives the model three anonymous images in a row
/// and no way to say which is which.
///
/// Never returns an error: a turn with one bad attachment should still be
/// sent, with the failure stated in-band rather than swallowed.
#[must_use]
pub fn expand_attachment_blocks(text: &str) -> ExpandedAttachments {
    let references = crate::tui::file_mention::media_attachment_references(text);
    let mut out = ExpandedAttachments::default();
    for reference in references {
        if reference.kind != "image" {
            // Video and any future kind: left as the text reference it
            // already was. Silently ignoring it here is not a drop — the
            // path is still in the prompt, exactly as before this module
            // existed.
            continue;
        }
        match attach_image_from_path(Path::new(&reference.path)) {
            Ok(image) => {
                out.blocks
                    .push(tag_block(&format!("<image path=\"{}\">", reference.path)));
                out.blocks.push(image.content_block());
                out.blocks.push(tag_block("</image>"));
            }
            Err(error) => out.notices.push(error.to_string()),
        }
    }
    out
}

fn tag_block(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_string(),
        cache_control: None,
    }
}

/// Replace every image in a request with text when the route cannot see them.
///
/// Capability is a property of the *route*, not of the attachment, and the
/// route changes freely mid-session. Deciding this when the message is built
/// would burn the answer into history: attach a screenshot while a text-only
/// model is selected, switch to a vision model, and the image would be gone
/// for good. So history always keeps the real image and each outbound request
/// is normalized against the model it is actually going to.
///
/// Only a known `Unsupported` strips. Most routes report `Unknown` because
/// models.dev has no modality data for them, and treating unknown as "no"
/// would make the feature dead on arrival for exactly the self-hosted and
/// custom routes that most need it — so `Unknown` sends the image and lets the
/// provider be the authority.
///
/// The image is replaced in place rather than removed, so the model is told
/// why it is looking at a gap instead of being left to invent one.
pub fn strip_images_when_unsupported(
    messages: &mut [crate::models::Message],
    vision: SupportState,
    model: &str,
) -> usize {
    if vision != SupportState::Unsupported {
        return 0;
    }
    let mut stripped = 0;
    for message in messages.iter_mut() {
        for block in &mut message.content {
            if matches!(block, ContentBlock::ImageUrl { .. }) {
                *block = ContentBlock::Text {
                    text: format!(
                        "[image content omitted: the active model ({model}) does \
                         not accept image input. Switch to a vision-capable \
                         model with /model to see it.]"
                    ),
                    cache_control: None,
                };
                stripped += 1;
            }
        }
    }
    stripped
}

/// Render dropped-attachment notices as a block the model will read.
///
/// Wrapped in a tag rather than appended as bare prose so the model can tell
/// the difference between the user saying something and the harness reporting
/// on itself.
#[must_use]
pub fn notice_block(notices: &[String]) -> Option<ContentBlock> {
    if notices.is_empty() {
        return None;
    }
    let body = notices.join("\n");
    Some(ContentBlock::Text {
        text: format!(
            "<attachment_notice>\n{body}\nDo not describe these images from \
             memory or from their filenames; ask the user to re-share them.\n\
             </attachment_notice>"
        ),
        cache_control: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1x1 PNG, as bytes rather than a fixture file so the encoding tests
    /// have no filesystem dependency.
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn sniffs_every_accepted_format_from_magic_bytes() {
        assert_eq!(sniff_media_type(PNG_1X1), Some("image/png"));
        assert_eq!(
            sniff_media_type(&[0xff, 0xd8, 0xff, 0xe0, 0x00]),
            Some("image/jpeg")
        );
        assert_eq!(sniff_media_type(b"GIF89a....."), Some("image/gif"));
        assert_eq!(sniff_media_type(b"GIF87a....."), Some("image/gif"));
        assert_eq!(
            sniff_media_type(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
    }

    #[test]
    fn sniffing_ignores_the_extension_and_believes_the_bytes() {
        // A JPEG named .png must be declared image/jpeg, or the provider
        // rejects the media-type mismatch.
        let jpeg = [0xff, 0xd8, 0xff, 0xe0, 0x11, 0x22];
        let attached = encode_image_bytes(&jpeg, "screenshot.png").expect("attach");
        assert_eq!(attached.media_type, "image/jpeg");
        assert!(attached.data_url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn riff_that_is_not_webp_is_not_an_image() {
        // A WAV file is also RIFF. Matching on "RIFF" alone would attach audio.
        assert_eq!(sniff_media_type(b"RIFF\x00\x00\x00\x00WAVEfmt "), None);
    }

    #[test]
    fn encodes_a_png_to_a_data_url_that_round_trips() {
        let attached = encode_image_bytes(PNG_1X1, "shot.png").expect("attach");
        assert_eq!(attached.media_type, "image/png");
        assert_eq!(attached.source_bytes, PNG_1X1.len());

        let (media_type, payload) = parse_data_url(&attached.data_url).expect("parse");
        assert_eq!(media_type, "image/png");
        assert_eq!(STANDARD.decode(payload).expect("decode"), PNG_1X1);
    }

    #[test]
    fn rejects_a_file_over_the_size_limit() {
        let oversized = vec![0u8; MAX_IMAGE_BYTES + 1];
        let error = encode_image_bytes(&oversized, "huge.png").expect_err("must reject");
        assert!(
            matches!(error, ImageAttachError::TooLarge { .. }),
            "got {error:?}"
        );
        let rendered = error.to_string();
        assert!(rendered.contains("5.0 MB"), "{rendered}");
        assert!(rendered.contains("huge.png"), "{rendered}");
    }

    #[test]
    fn accepts_a_file_exactly_at_the_size_limit() {
        // The boundary is inclusive; an off-by-one here would reject images
        // the providers accept.
        let mut at_limit = PNG_1X1.to_vec();
        at_limit.resize(MAX_IMAGE_BYTES, 0);
        assert!(encode_image_bytes(&at_limit, "edge.png").is_ok());
    }

    #[test]
    fn rejects_a_real_image_in_an_unsupported_format_by_name() {
        for (bytes, name) in [
            (b"BM\x00\x00\x00\x00".as_slice(), "BMP"),
            (b"II\x2a\x00extra".as_slice(), "TIFF"),
            (b"MM\x00\x2aextra".as_slice(), "TIFF"),
            (b"<svg xmlns=".as_slice(), "SVG"),
            (b"%PDF-1.7".as_slice(), "PDF"),
        ] {
            let error = encode_image_bytes(bytes, "f").expect_err("must reject");
            match error {
                ImageAttachError::UnsupportedFormat { detected, .. } => {
                    assert_eq!(detected, name);
                }
                other => panic!("expected UnsupportedFormat for {name}, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_a_file_that_is_not_an_image_at_all() {
        let error =
            encode_image_bytes(b"#!/bin/sh\necho hi\n", "script.png").expect_err("must reject");
        assert!(
            matches!(error, ImageAttachError::NotAnImage { .. }),
            "got {error:?}"
        );
    }

    #[test]
    fn rejects_an_empty_file() {
        let error = encode_image_bytes(b"", "empty.png").expect_err("must reject");
        assert!(
            matches!(error, ImageAttachError::Empty { .. }),
            "got {error:?}"
        );
    }

    #[test]
    fn parses_and_rejects_data_urls() {
        assert_eq!(
            parse_data_url("data:image/png;base64,QUJD"),
            Some(("image/png", "QUJD"))
        );
        // Not base64-tagged: Anthropic has no shape for a raw data URL.
        assert_eq!(parse_data_url("data:image/png,QUJD"), None);
        // Remote URLs are a different source type, not a malformed data URL.
        assert_eq!(parse_data_url("https://example.com/a.png"), None);
        // Degenerate forms must not produce an empty base64 payload that the
        // provider would reject with an opaque error.
        assert_eq!(parse_data_url("data:;base64,QUJD"), None);
        assert_eq!(parse_data_url("data:image/png;base64,"), None);
        assert_eq!(parse_data_url("data:image/png;base64"), None);
    }

    #[test]
    fn classifies_remote_urls() {
        assert!(is_remote_image_url("https://example.com/a.png"));
        assert!(is_remote_image_url("http://example.com/a.png"));
        assert!(!is_remote_image_url("data:image/png;base64,QUJD"));
        assert!(!is_remote_image_url("file:///tmp/a.png"));
    }

    fn message_with_image(url: &str) -> crate::models::Message {
        crate::models::Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::ImageUrl {
                    image_url: ImageUrlContent {
                        url: url.to_string(),
                    },
                },
                ContentBlock::Text {
                    text: "what is this?".to_string(),
                    cache_control: None,
                },
            ],
        }
    }

    fn write_png(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, PNG_1X1).expect("write fixture");
        path
    }

    #[test]
    fn expands_a_placeholder_into_an_image_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_png(dir.path(), "shot.png");
        let text = format!("look at this\n[Attached image: {}]", path.display());

        let expanded = expand_attachment_blocks(&text);

        assert!(expanded.notices.is_empty(), "{expanded:?}");
        // Bracketed: open tag naming the path, the image, close tag.
        assert_eq!(expanded.blocks.len(), 3, "{expanded:?}");
        match &expanded.blocks[0] {
            ContentBlock::Text { text, .. } => {
                assert!(text.starts_with("<image path=\""), "{text}");
                assert!(text.contains("shot.png"), "{text}");
            }
            other => panic!("expected an opening tag, got {other:?}"),
        }
        match &expanded.blocks[1] {
            ContentBlock::ImageUrl { image_url } => {
                assert!(image_url.url.starts_with("data:image/png;base64,"));
            }
            other => panic!("expected an image block, got {other:?}"),
        }
        assert_eq!(
            expanded.blocks[2],
            ContentBlock::Text {
                text: "</image>".to_string(),
                cache_control: None
            }
        );
    }

    #[test]
    fn expands_multiple_placeholders_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = write_png(dir.path(), "one.png");
        let second = write_png(dir.path(), "two.png");
        std::fs::write(&second, [0xff, 0xd8, 0xff, 0xe0, 0x01]).expect("write jpeg");
        let text = format!(
            "[Attached image: {}]\nand\n[Attached image: {}]",
            first.display(),
            second.display()
        );

        let expanded = expand_attachment_blocks(&text);

        let media: Vec<_> = expanded
            .blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ImageUrl { image_url } => Some(
                    parse_data_url(&image_url.url)
                        .expect("data url")
                        .0
                        .to_string(),
                ),
                _ => None,
            })
            .collect();
        assert_eq!(media, vec!["image/png", "image/jpeg"]);

        // Each image carries its own path tag, so the model can tell two
        // screenshots in one turn apart.
        let tags: Vec<_> = expanded
            .blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text, .. } if text.starts_with("<image path=") => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(tags.len(), 2, "{tags:?}");
        assert!(tags[0].contains("one.png"), "{tags:?}");
        assert!(tags[1].contains("two.png"), "{tags:?}");
    }

    #[test]
    fn ingest_does_not_consult_model_capability() {
        // Capability is a route property and is re-decided per request. If
        // ingest started gating on it, attaching under a text-only model would
        // destroy the image for the rest of the session.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_png(dir.path(), "shot.png");
        let text = format!("[Attached image: {}]", path.display());

        let expanded = expand_attachment_blocks(&text);

        assert_eq!(expanded.blocks.len(), 3);
        assert!(expanded.notices.is_empty());
    }

    #[test]
    fn a_blind_route_gets_text_in_place_of_every_image() {
        let mut messages = vec![
            message_with_image("data:image/png;base64,QUJD"),
            crate::models::Message {
                role: "assistant".to_string(),
                content: vec![ContentBlock::Text {
                    text: "sure".to_string(),
                    cache_control: None,
                }],
            },
        ];

        let stripped = strip_images_when_unsupported(
            &mut messages,
            SupportState::Unsupported,
            "deepseek-chat",
        );

        assert_eq!(stripped, 1);
        assert!(
            !messages[0]
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::ImageUrl { .. })),
            "no image may survive to a route that cannot read one"
        );
        match &messages[0].content[0] {
            ContentBlock::Text { text, .. } => {
                assert!(text.contains("deepseek-chat"), "{text}");
                assert!(text.contains("/model"), "{text}");
                assert!(text.contains("omitted"), "{text}");
            }
            other => panic!("expected replacement text, got {other:?}"),
        }
    }

    #[test]
    fn a_supported_or_unknown_route_keeps_its_images() {
        // Unknown is the common case: models.dev has no modality data for most
        // routes. Stripping there would make the feature dead on arrival for
        // self-hosted and custom providers.
        for vision in [SupportState::Supported, SupportState::Unknown] {
            let mut messages = vec![message_with_image("data:image/png;base64,QUJD")];

            let stripped = strip_images_when_unsupported(&mut messages, vision, "some-model");

            assert_eq!(stripped, 0, "{vision:?} must not strip");
            assert!(
                messages[0]
                    .content
                    .iter()
                    .any(|block| matches!(block, ContentBlock::ImageUrl { .. })),
                "{vision:?} must keep the image"
            );
        }
    }

    #[test]
    fn stripping_replaces_every_image_across_every_message() {
        let mut messages = vec![
            message_with_image("data:image/png;base64,AAAA"),
            message_with_image("data:image/jpeg;base64,BBBB"),
        ];

        let stripped =
            strip_images_when_unsupported(&mut messages, SupportState::Unsupported, "blind");

        assert_eq!(
            stripped, 2,
            "a per-message early return would miss the second"
        );
    }

    #[test]
    fn a_missing_file_becomes_a_notice_not_a_dropped_turn() {
        let text = "[Attached image: /nonexistent/definitely-not-here.png]";

        let expanded = expand_attachment_blocks(text);

        assert!(expanded.blocks.is_empty());
        assert_eq!(expanded.notices.len(), 1);
        assert!(
            expanded.notices[0].contains("definitely-not-here.png"),
            "{:?}",
            expanded.notices
        );
    }

    #[test]
    fn one_bad_attachment_does_not_suppress_a_good_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = write_png(dir.path(), "good.png");
        let text = format!(
            "[Attached image: /nope/missing.png]\n[Attached image: {}]",
            good.display()
        );

        let expanded = expand_attachment_blocks(&text);

        assert_eq!(expanded.blocks.len(), 3);
        assert_eq!(expanded.notices.len(), 1);
    }

    #[test]
    fn video_attachments_are_left_as_text() {
        let text = "[Attached video: /tmp/clip.mp4]";

        let expanded = expand_attachment_blocks(text);

        assert!(expanded.blocks.is_empty(), "{expanded:?}");
        assert!(expanded.notices.is_empty(), "{expanded:?}");
    }

    #[test]
    fn text_with_no_attachments_produces_nothing() {
        let expanded = expand_attachment_blocks("just a normal question");
        assert!(expanded.blocks.is_empty());
        assert!(expanded.notices.is_empty());
    }

    #[test]
    fn notice_block_names_the_failure_and_forbids_guessing() {
        assert_eq!(notice_block(&[]), None);
        let block =
            notice_block(&["Cannot attach a.png: the file is empty".to_string()]).expect("block");
        match block {
            ContentBlock::Text { text, .. } => {
                assert!(text.contains("<attachment_notice>"), "{text}");
                assert!(text.contains("a.png"), "{text}");
                assert!(text.contains("Do not describe"), "{text}");
            }
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn attach_from_path_reports_an_unreadable_file() {
        let error = attach_image_from_path(Path::new("/nonexistent/x.png")).expect_err("must fail");
        assert!(
            matches!(error, ImageAttachError::Unreadable { .. }),
            "got {error:?}"
        );
    }
}
