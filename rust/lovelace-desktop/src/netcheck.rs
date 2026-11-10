//! Structural proof that the desktop binary cannot open a socket.
//!
//! The desktop application replaces an HTTP server, so "no TCP listener" is the defining
//! property of the product and has to be checked, not asserted in prose. Two checks live here:
//!
//! 1. imported_dlls walks the PE import directory of a real image (including this binary) using
//!    only integer reads, and network_imports flags any DLL that would bring socket APIs with it.
//!    --selftest runs this against its own executable, so a networking import could not pass.
//! 2. A unit test walks this crate's sources and rejects any reference to std::net, a listener,
//!    a stream, a UDP socket or the Winsock startup call. netcheck.rs itself is skipped because
//!    it necessarily contains those words in the deny list above.
//!
//! There is no networking dependency in Cargo.toml either; the crate depends on lovelace-json and
//! (optionally, off by default) lovelace-lang only.

use std::path::Path;

/// DLLs whose presence in an import table means the image can open sockets.
pub const NETWORK_DLLS: &[&str] = &[
    "ws2_32.dll",
    "wsock32.dll",
    "mswsock.dll",
    "winhttp.dll",
    "wininet.dll",
    "urlmon.dll",
    "dnsapi.dll",
];

/// One PE section, as needed to convert RVAs to file offsets.
struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_size: u32,
    raw_offset: u32,
}

/// Returns every DLL name in a PE image's import directory. Fails only when the bytes are not a
/// PE image this reader understands.
pub fn imported_dlls(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() < 0x40 {
        return Err("file is too small to be a PE image".to_string());
    }
    if bytes.get(0..2) != Some(&b"MZ"[..]) {
        return Err("missing MZ signature".to_string());
    }
    let pe = read_u32(bytes, 0x3c)? as usize;
    if bytes.len() < pe + 24 {
        return Err("truncated DOS header".to_string());
    }
    if bytes.get(pe..pe + 4) != Some(&b"PE\0\0"[..]) {
        return Err("missing PE signature".to_string());
    }
    let coff = pe + 4;
    let section_count = read_u16(bytes, coff + 2)? as usize;
    let optional_size = read_u16(bytes, coff + 16)? as usize;
    let optional = coff + 20;
    let magic = read_u16(bytes, optional)?;
    let (directories, directory_count_at) = match magic {
        0x20b => (optional + 112, optional + 108),
        0x10b => (optional + 96, optional + 92),
        other => return Err(format!("unknown PE optional header magic 0x{other:x}")),
    };
    let directory_count = read_u32(bytes, directory_count_at)? as usize;
    if directory_count < 2 {
        return Ok(Vec::new());
    }
    let import_rva = read_u32(bytes, directories + 8)?;
    let import_size = read_u32(bytes, directories + 12)?;
    if import_rva == 0 || import_size == 0 {
        return Ok(Vec::new());
    }
    let sections = read_sections(bytes, coff, optional_size, section_count)?;
    let mut offset = rva_to_offset(import_rva, &sections)?;
    let mut names = Vec::new();
    for _ in 0..4096 {
        let original_thunk = read_u32(bytes, offset)?;
        let name_rva = read_u32(bytes, offset + 12)?;
        let first_thunk = read_u32(bytes, offset + 16)?;
        if original_thunk == 0 && name_rva == 0 && first_thunk == 0 {
            break;
        }
        if name_rva != 0 {
            let name_offset = rva_to_offset(name_rva, &sections)?;
            names.push(read_cstring(bytes, name_offset)?);
        }
        offset += 20;
    }
    Ok(names)
}

/// Reads the import directory of a file on disk.
pub fn file_imports(path: &Path) -> Result<Vec<String>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    imported_dlls(&bytes)
}

/// Returns the subset of names that can open sockets.
pub fn network_imports(dlls: &[String]) -> Vec<String> {
    dlls.iter()
        .filter(|name| {
            let lowered = name.to_ascii_lowercase();
            NETWORK_DLLS.contains(&lowered.as_str())
        })
        .cloned()
        .collect()
}

fn read_u16(bytes: &[u8], at: usize) -> Result<u16, String> {
    let slice = bytes
        .get(at..at + 2)
        .ok_or_else(|| format!("truncated image at byte {at}"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or_else(|| format!("truncated image at byte {at}"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_sections(
    bytes: &[u8],
    coff: usize,
    optional_size: usize,
    count: usize,
) -> Result<Vec<Section>, String> {
    let table = coff + 20 + optional_size;
    let mut sections = Vec::with_capacity(count);
    for index in 0..count {
        let at = table + index * 40;
        sections.push(Section {
            virtual_size: read_u32(bytes, at + 8)?,
            virtual_address: read_u32(bytes, at + 12)?,
            raw_size: read_u32(bytes, at + 16)?,
            raw_offset: read_u32(bytes, at + 20)?,
        });
    }
    Ok(sections)
}

fn rva_to_offset(rva: u32, sections: &[Section]) -> Result<usize, String> {
    for section in sections {
        let span = section.virtual_size.max(section.raw_size);
        if rva >= section.virtual_address && rva < section.virtual_address.saturating_add(span) {
            let delta = rva - section.virtual_address;
            if delta >= section.raw_size {
                break;
            }
            return Ok((section.raw_offset + delta) as usize);
        }
    }
    Err(format!("RVA 0x{rva:x} is not backed by any section"))
}

fn read_cstring(bytes: &[u8], at: usize) -> Result<String, String> {
    let tail = bytes
        .get(at..)
        .ok_or_else(|| format!("truncated image at byte {at}"))?;
    let end = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| format!("unterminated import name at byte {at}"))?;
    String::from_utf8(tail[..end].to_vec()).map_err(|_| "import name is not UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Builds a minimal PE32+ image with one import descriptor naming dll.
    fn synthetic_pe(dll: &str) -> Vec<u8> {
        let mut image = vec![0u8; 0x400];
        image[0] = b'M';
        image[1] = b'Z';
        image[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        image[0x40..0x44].copy_from_slice(b"PE\0\0");
        let coff = 0x44usize;
        image[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
        image[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
        image[coff + 16..coff + 18].copy_from_slice(&0xf0u16.to_le_bytes());
        let optional = coff + 20;
        image[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
        image[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
        let directories = optional + 112;
        image[directories + 8..directories + 12].copy_from_slice(&0x1000u32.to_le_bytes());
        image[directories + 12..directories + 16].copy_from_slice(&40u32.to_le_bytes());
        let section = optional + 0xf0;
        image[section..section + 6].copy_from_slice(b".rdata");
        image[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
        image[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        image[section + 16..section + 20].copy_from_slice(&0x200u32.to_le_bytes());
        image[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());

        let descriptor = 0x200usize;
        image[descriptor + 12..descriptor + 16].copy_from_slice(&0x1100u32.to_le_bytes());
        image[descriptor + 16..descriptor + 20].copy_from_slice(&0x1120u32.to_le_bytes());

        let name = 0x300usize;
        image[name..name + dll.len()].copy_from_slice(dll.as_bytes());
        image[name + dll.len()] = 0;
        image
    }

    #[test]
    fn reads_the_import_directory_of_a_synthetic_image() {
        let image = synthetic_pe("WS2_32.dll");
        assert_eq!(imported_dlls(&image).unwrap(), vec!["WS2_32.dll".to_string()]);
        assert_eq!(
            network_imports(&imported_dlls(&image).unwrap()),
            vec!["WS2_32.dll".to_string()]
        );
    }

    #[test]
    fn a_harmless_dll_is_not_flagged() {
        let image = synthetic_pe("KERNEL32.dll");
        let imports = imported_dlls(&image).unwrap();
        assert_eq!(imports, vec!["KERNEL32.dll".to_string()]);
        assert!(network_imports(&imports).is_empty());
    }

    #[test]
    fn rejects_non_pe_input() {
        assert!(imported_dlls(b"not an executable").is_err());
        assert!(imported_dlls(&[]).is_err());
    }

    #[test]
    fn this_test_binary_imports_no_networking_dll() {
        let executable = std::env::current_exe().expect("current executable");
        let imports = file_imports(&executable).expect("PE imports");
        assert!(
            !imports.is_empty(),
            "expected kernel32 and friends, got no imports at all"
        );
        assert!(
            network_imports(&imports).is_empty(),
            "networking imports found: {:?}",
            network_imports(&imports)
        );
    }

    #[test]
    fn crate_sources_never_reference_networking_apis() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rust_files(&root, &mut files);
        assert!(files.len() > 4, "expected to scan the crate sources, found {}", files.len());
        let needles = [
            "TcpListener",
            "TcpStream",
            "UdpSocket",
            "std::net",
            "WSAStartup",
            "ws2_32",
            "getaddrinfo",
        ];
        let mut hits: Vec<String> = Vec::new();
        for file in files {
            if file.file_name().map(|name| name == "netcheck.rs").unwrap_or(false) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            for needle in needles {
                if text.contains(needle) {
                    hits.push(format!("{} contains {needle}", file.display()));
                }
            }
        }
        assert!(hits.is_empty(), "networking references found: {hits:?}");
    }

    fn collect_rust_files(directory: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rust_files(&path, out);
            } else if path.extension().map(|extension| extension == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
}
