//! README presentation contracts (band 188).
//!
//! GitHub plays SMIL inside standalone SVG `<img>` files. The old PNG tiles
//! were never committed; these files must stay in-tree and script-free.

use std::fs;
use std::path::PathBuf;

fn kit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const PRESENTATIONS: &[&str] = &["gsv-hero.svg", "gsv-install.svg", "gsv-flow.svg"];

#[test]
fn readme_embeds_smil_presentations() {
    let root = kit_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("README.md");
    assert!(
        !readme.contains("gsv-hero.png"),
        "README must not reference missing PNG hero"
    );
    for name in PRESENTATIONS {
        assert!(
            readme.contains(name),
            "README.md must embed docs/assets/presentations/{name}"
        );
        let path = root.join("docs/assets/presentations").join(name);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
        assert!(
            bytes.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
            "{name} must start with an XML declaration so GitHub can parse it"
        );
        for (i, b) in bytes.iter().copied().enumerate() {
            assert!(
                b >= 32 || matches!(b, b'\t' | b'\n' | b'\r'),
                "{name} has illegal XML byte {b:#04x} at {i} (GitHub: Invalid image source)"
            );
        }
        let svg = std::str::from_utf8(&bytes).unwrap_or_else(|e| panic!("{name} UTF-8: {e}"));
        assert!(
            svg.contains("http://www.w3.org/2000/svg"),
            "{name} missing SVG xmlns"
        );
        assert!(
            svg.contains("<animate") || svg.contains("<animateTransform"),
            "{name} must use SMIL so GitHub README <img> can play it"
        );
        let lower = svg.to_ascii_lowercase();
        assert!(
            !lower.contains("<script"),
            "{name} must not contain <script>"
        );
        assert!(
            !lower.contains("<filter"),
            "{name} must not use SVG filters (GitHub blob preview rejects them)"
        );
    }
}

#[test]
fn presentations_readme_names_smil_canon() {
    let text = fs::read_to_string(kit_root().join("docs/assets/presentations/README.md"))
        .expect("presentations README");
    assert!(text.contains("SMIL"), "{text}");
    assert!(text.contains("svg-motion-cookbook"), "{text}");
}
