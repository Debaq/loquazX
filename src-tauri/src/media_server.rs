//! Servidor HTTP local para servir media al webview (ADR-005).
//!
//! WebKitGTK entrega las URI de media a GStreamer sin pasar por los handlers
//! de schemes custom, así que el protocolo `asset:` no sirve para video en
//! Linux. GStreamer sí reproduce HTTP local, y el soporte de `Range` habilita
//! el seek sin cargar el archivo en memoria.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct MediaServer {
    port: u16,
    token: String,
    allowed: Arc<Mutex<HashSet<PathBuf>>>,
}

impl MediaServer {
    /// Arranca el servidor en 127.0.0.1 con puerto efímero y token aleatorio.
    pub fn start() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("No se pudo iniciar el servidor de media: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("No se pudo obtener el puerto del servidor de media: {e}"))?
            .port();
        let token = uuid::Uuid::new_v4().simple().to_string();
        let allowed: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

        let token_accept = token.clone();
        let allowed_accept = Arc::clone(&allowed);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let token = token_accept.clone();
                let allowed = Arc::clone(&allowed_accept);
                std::thread::spawn(move || {
                    let _ = handle_connection(stream, &token, &allowed);
                });
            }
        });

        Ok(Self {
            port,
            token,
            allowed,
        })
    }

    /// Registra el archivo en la allowlist y devuelve su URL local.
    pub fn url_for(&self, file: &Path) -> Result<String, String> {
        let canonical = file
            .canonicalize()
            .map_err(|e| format!("No se pudo resolver la ruta del media: {e}"))?;
        if !canonical.is_file() {
            return Err(format!("El media no existe: {}", canonical.display()));
        }
        let url = format!(
            "http://127.0.0.1:{}/{}/{}",
            self.port,
            self.token,
            percent_encode(&canonical.to_string_lossy())
        );
        self.allowed
            .lock()
            .map_err(|_| "Allowlist del servidor de media envenenada".to_string())?
            .insert(canonical);
        Ok(url)
    }
}

fn handle_connection(
    stream: TcpStream,
    token: &str,
    allowed: &Mutex<HashSet<PathBuf>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();

    // Solo interesa el header Range; el resto se consume hasta la línea vacía.
    let mut range_header = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range:") {
            range_header = Some(value.trim().to_string());
        }
    }
    let mut stream = reader.into_inner();

    if method != "GET" && method != "HEAD" {
        return respond_error(&mut stream, "405 Method Not Allowed");
    }
    let prefix = format!("/{token}/");
    let Some(encoded) = target.strip_prefix(&prefix) else {
        return respond_error(&mut stream, "403 Forbidden");
    };
    let Some(decoded) = percent_decode(encoded) else {
        return respond_error(&mut stream, "400 Bad Request");
    };
    // Canonizar antes de consultar la allowlist neutraliza `..` y symlinks.
    let Ok(path) = Path::new(&decoded).canonicalize() else {
        return respond_error(&mut stream, "404 Not Found");
    };
    let permitted = allowed.lock().map(|a| a.contains(&path)).unwrap_or(false);
    if !permitted {
        return respond_error(&mut stream, "403 Forbidden");
    }
    let Ok(mut file) = File::open(&path) else {
        return respond_error(&mut stream, "404 Not Found");
    };
    let length = file.metadata()?.len();

    let range = range_header.as_deref().map(|r| parse_range(r, length));
    let (status, start, end) = match range {
        None => ("200 OK", 0, length.saturating_sub(1)),
        Some(Some((start, end))) => ("206 Partial Content", start, end),
        Some(None) => {
            let header = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{length}\r\nConnection: close\r\n\r\n"
            );
            return stream.write_all(header.as_bytes());
        }
    };

    let body_length = if length == 0 { 0 } else { end - start + 1 };
    let mut header = format!(
        "HTTP/1.1 {status}\r\nAccept-Ranges: bytes\r\nContent-Type: {}\r\nContent-Length: {body_length}\r\nConnection: close\r\n",
        content_type(&path)
    );
    if status.starts_with("206") {
        header.push_str(&format!("Content-Range: bytes {start}-{end}/{length}\r\n"));
    }
    header.push_str("\r\n");
    stream.write_all(header.as_bytes())?;

    if method == "GET" && body_length > 0 {
        file.seek(SeekFrom::Start(start))?;
        std::io::copy(&mut file.take(body_length), &mut stream)?;
    }
    Ok(())
}

fn respond_error(stream: &mut TcpStream, status: &str) -> std::io::Result<()> {
    let header = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(header.as_bytes())
}

/// `bytes=a-b` o `bytes=a-`; None si el rango no es satisfacible.
fn parse_range(header: &str, length: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?;
    let (start, end) = spec.split_once('-')?;
    let start: u64 = start.trim().parse().ok()?;
    let end: u64 = match end.trim() {
        "" => length.checked_sub(1)?,
        e => e.parse().ok()?,
    };
    if start > end || end >= length {
        return None;
    }
    Some((start, end))
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Petición HTTP cruda; devuelve (línea de estado, headers, cuerpo).
    fn request(port: u16, target: &str, range: Option<&str>) -> (String, String, Vec<u8>) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let extra = range.map_or(String::new(), |r| format!("Range: {r}\r\n"));
        stream
            .write_all(format!("GET {target} HTTP/1.1\r\nHost: x\r\n{extra}\r\n").as_bytes())
            .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).to_string();
        let (status, headers) = head.split_once("\r\n").unwrap_or((&head, ""));
        (
            status.to_string(),
            headers.to_string(),
            raw[split + 4..].to_vec(),
        )
    }

    fn server_with_file() -> (MediaServer, std::path::PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vídeo de prueba.mp4");
        std::fs::write(&file, (0..=255u8).cycle().take(1000).collect::<Vec<_>>()).unwrap();
        (MediaServer::start().unwrap(), file, dir)
    }

    fn target_of(server: &MediaServer, file: &Path) -> String {
        let url = server.url_for(file).unwrap();
        let after_scheme = url.strip_prefix("http://").unwrap();
        after_scheme[after_scheme.find('/').unwrap()..].to_string()
    }

    #[test]
    fn sirve_el_archivo_completo() {
        let (server, file, _dir) = server_with_file();
        let (status, headers, body) = request(server.port, &target_of(&server, &file), None);
        assert!(status.contains("200"), "status: {status}");
        assert!(headers.contains("Accept-Ranges: bytes"));
        assert!(headers.contains("Content-Type: video/mp4"));
        assert_eq!(body.len(), 1000);
        assert_eq!(body[..4], [0, 1, 2, 3]);
    }

    #[test]
    fn sirve_rangos_parciales() {
        let (server, file, _dir) = server_with_file();
        let target = target_of(&server, &file);
        let (status, headers, body) = request(server.port, &target, Some("bytes=10-19"));
        assert!(status.contains("206"), "status: {status}");
        assert!(headers.contains("Content-Range: bytes 10-19/1000"));
        assert_eq!(body, (10..=19u8).collect::<Vec<_>>());

        let (status, _, body) = request(server.port, &target, Some("bytes=990-"));
        assert!(status.contains("206"), "status: {status}");
        assert_eq!(body.len(), 10);
    }

    #[test]
    fn rechaza_rango_fuera_del_archivo() {
        let (server, file, _dir) = server_with_file();
        let (status, _, _) = request(server.port, &target_of(&server, &file), Some("bytes=2000-"));
        assert!(status.contains("416"), "status: {status}");
    }

    #[test]
    fn rechaza_token_invalido() {
        let (server, file, _dir) = server_with_file();
        let target = target_of(&server, &file).replacen(&server.token, "tokenfalso", 1);
        let (status, _, _) = request(server.port, &target, None);
        assert!(status.contains("403"), "status: {status}");
    }

    #[test]
    fn rechaza_archivo_no_registrado() {
        let (server, file, _dir) = server_with_file();
        let intruso = file.parent().unwrap().join("secreto.txt");
        std::fs::write(&intruso, b"privado").unwrap();
        let registered = target_of(&server, &file);
        let target = registered.replace(
            &percent_encode(&file.canonicalize().unwrap().to_string_lossy()),
            &percent_encode(&intruso.canonicalize().unwrap().to_string_lossy()),
        );
        let (status, _, _) = request(server.port, &target, None);
        assert!(status.contains("403"), "status: {status}");
    }

    #[test]
    fn url_para_archivo_inexistente_falla() {
        let (server, _file, dir) = server_with_file();
        assert!(server.url_for(&dir.path().join("nada.mp4")).is_err());
    }
}
