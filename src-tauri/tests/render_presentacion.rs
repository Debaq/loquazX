//! Test de integración end-to-end del render de presentación (ADR-010).
//! Marcado `#[ignore]` porque asume `ffmpeg`, `pdftoppm` y `pdfinfo` en el
//! PATH (mismo patrón que los tests que asumen `ffmpeg` en `project::tests`).
//! Se ejecuta con `cargo test -- --ignored`.

use std::path::PathBuf;
use std::process::Command;

fn ffmpeg_disponible() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn poppler_disponible() -> bool {
    Command::new("pdftoppm")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        && Command::new("pdfinfo")
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Genera un PDF de 3 páginas con reportlab (si está disponible) y devuelve su
/// ruta. Falla el test si reportlab no está. Construye el script con `"\n"`
/// explícitos: dentro de un string literal con `\<nl>`, Rust elimina tanto el
/// newline como el whitespace inicial de la línea siguiente.
fn generar_pdf(dir: &std::path::Path) -> PathBuf {
    let pdf = dir.join("fuente.pdf");
    let script_path = dir.join("gen_pdf.py");
    let body = [
        "from reportlab.pdfgen import canvas",
        "from reportlab.lib.pagesizes import letter",
        &format!("c = canvas.Canvas('{}', pagesize=letter)", pdf.display()),
        "for t in ['Pagina 1', 'Pagina 2', 'Pagina 3']:",
        "    c.setFont('Helvetica-Bold', 48)",
        "    c.drawCentredString(letter[0]/2, letter[1]/2, t)",
        "    c.showPage()",
        "c.save()",
        "",
    ]
    .join("\n");
    std::fs::write(&script_path, body).unwrap();
    let status = Command::new("python3")
        .arg(&script_path)
        .status()
        .expect("python3 debe estar instalado");
    assert!(
        status.success(),
        "no se pudo generar el PDF con reportlab (¿está instalado?)"
    );
    pdf
}

#[test]
#[ignore]
fn render_presentacion_e2e() {
    if !ffmpeg_disponible() || !poppler_disponible() {
        eprintln!("Falta ffmpeg, pdftoppm o pdfinfo; saltando.");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("demo.lqzx");

    // Crea el proyecto manualmente: la API interna `project::create` no es
    // pública, así que escribimos `project.json` y `segments.json` a mano.
    std::fs::create_dir_all(&project).unwrap();
    let manifest = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "format_version": 1,
        "name": "Demo",
        "source_language": "es",
        "target_language": "en",
        "created_at": 0,
    });
    std::fs::write(
        project.join("project.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(project.join("segments.json"), "{\"segments\":[]}").unwrap();
    for sub in ["source", "media", "runs", "exports", "slides"] {
        std::fs::create_dir_all(project.join(sub)).unwrap();
    }

    // Importa el PDF usando el comando Tauri vía el módulo público. Esto
    // también rasteriza las páginas bajo `slides/pages/`, que es lo que usa
    // tanto el preview como el render final.
    let pdf = generar_pdf(dir.path());
    let _ = loquazx_lib::__test::importar_pdf(&project, &pdf).expect("importar PDF");

    // Verifica que las páginas se rasterizaron al importar.
    for n in 1..=3 {
        let png = project.join("slides").join("pages").join(format!("page-{n}.png"));
        assert!(png.is_file(), "falta la página rasterizada: {}", png.display());
    }

    // Importa los segmentos.
    let json = dir.path().join("segs.json");
    std::fs::write(
        &json,
        r#"{"segments":[
            {"start":0.0,"end":2.0,"source":"Hola","slide":1},
            {"start":2.5,"end":4.0,"source":"Mundo","slide":2},
            {"start":4.5,"end":6.0,"source":"Adios","slide":3}
        ]}"#,
    )
    .unwrap();
    let _ =
        loquazx_lib::__test::importar_segmentos_json(&project, &json).expect("importar segmentos");

    // Genera WAVs dummy de 1.5 s por segmento.
    let segments = loquazx_lib::__test::load_segments(&project).expect("load_segments");
    assert_eq!(segments.len(), 3);
    let dub_dir = project.join("runs").join("dub");
    std::fs::create_dir_all(&dub_dir).unwrap();
    for s in &segments {
        let wav = dub_dir.join(format!("{}.wav", s.id));
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1.5",
                "-ar",
                "16000",
                "-ac",
                "1",
                wav.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success(), "no se pudo generar {}", wav.display());
    }

    // Render.
    let report =
        loquazx_lib::__test::render_presentacion(&project, |_, _| {}).expect("render_presentacion");
    let out = PathBuf::from(&report.output);
    assert!(out.is_file(), "no se generó el mp4: {}", out.display());
    assert!(
        report.duration_secs > 5.0,
        "duración demasiado corta: {}",
        report.duration_secs
    );

    // Verifica que el mp4 tiene pista de video h264.
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe");
    assert!(probe.status.success(), "ffprobe falló");
    let codec = String::from_utf8_lossy(&probe.stdout);
    assert_eq!(codec.trim(), "h264", "codec de video inesperado: {codec}");

    // Auto-recuperación (ADR-010): simula un cierre abrupto durante el import
    // borrando `slides/pages/` y dejando solo el PDF. Al volver a abrir el
    // proyecto, `open` debe regenerar las imágenes.
    let pages = project.join("slides").join("pages");
    std::fs::remove_dir_all(&pages).expect("borrar pages/ para simular cierre abrupto");
    assert!(!pages.exists(), "pages/ debería estar borrado");

    let _ = loquazx_lib::__test::abrir(&project).expect("reabrir");
    for n in 1..=3 {
        let png = pages.join(format!("page-{n}.png"));
        assert!(
            png.is_file(),
            "auto-recuperación no regeneró la página {n}: {}",
            png.display()
        );
    }

    // El MediaServer debe poder servir las páginas rasterizadas (es el camino
    // que usa el frontend para el preview; si falla aquí, el preview falla en
    // la app). Verificamos con la página 1.
    media_server_sirve_pagina(&pages.join("page-1.png"));

    // Reproduce el flujo exacto del frontend: dado un `Project` recién
    // abierto, construir la ruta `${project.path}/slides/pages/page-1.png`
    // como hace `VideoPreview`, y pedir la URL al MediaServer.
    let project_state = loquazx_lib::__test::abrir(&project).expect("abrir proyecto");
    eprintln!("[debug] project.path       = {:?}", project_state.path);
    eprintln!("[debug] project.slides_path = {:?}", project_state.slides_path);
    eprintln!("[debug] pages/page-1.png existe = {}", pages.join("page-1.png").is_file());
    let ruta_frontend = format!(
        "{}/slides/pages/page-1.png",
        project_state.path
    );
    eprintln!("[debug] ruta construida por el frontend = {ruta_frontend}");
    eprintln!("[debug] ruta construida existe en disco = {}", PathBuf::from(&ruta_frontend).is_file());
    media_server_sirve_pagina(&PathBuf::from(&ruta_frontend));
}

/// Variante del E2E con un path que tiene espacios y acentos, que es donde
/// sospechamos que el bug se manifiesta. Si este test pasa pero el del usuario
/// falla, el problema es específico a su entorno (no reproducible aquí).
#[test]
#[ignore]
fn render_presentacion_path_con_espacios_y_acentos() {
    if !ffmpeg_disponible() || !poppler_disponible() {
        eprintln!("Falta ffmpeg, pdftoppm o pdfinfo; saltando.");
        return;
    }
    let dir = tempfile::Builder::new()
        .prefix("demo con espacios y acentos ñ ")
        .tempdir()
        .unwrap();
    let project = dir.path().join("proyecto.lqzx");
    let _ = loquazx_lib::__test::crear_proyecto(&project, "Demo", "es", "en")
        .expect("crear proyecto");

    let pdf = generar_pdf(dir.path());
    let _ = loquazx_lib::__test::importar_pdf(&project, &pdf).expect("importar PDF");

    let project_state = loquazx_lib::__test::abrir(&project).expect("abrir proyecto");
    eprintln!("[debug-acentos] project.path = {:?}", project_state.path);
    let ruta_frontend = format!(
        "{}/slides/pages/page-1.png",
        project_state.path
    );
    eprintln!("[debug-acentos] ruta frontend = {ruta_frontend}");
    assert!(
        PathBuf::from(&ruta_frontend).is_file(),
        "imagen no existe en disco bajo path con espacios/acentos"
    );
    media_server_sirve_pagina(&PathBuf::from(&ruta_frontend));
}

/// Verifica que el `MediaServer` puede registrar y servir un archivo por HTTP.
/// Es el mismo flujo que `VideoPreview` usa en el frontend para mostrar la
/// diapositiva activa.
fn media_server_sirve_pagina(png: &std::path::Path) {
    use std::io::{Read, Write};
    let server = loquazx_lib::__test::iniciar_media_server().expect("iniciar MediaServer");
    let url = server
        .url_for(png)
        .unwrap_or_else(|e| panic!("url_for falló para {}: {e}", png.display()));
    let path = url.strip_prefix("http://127.0.0.1:").unwrap();
    let Some(target) = path.find('/').map(|i| &path[i..]) else {
        panic!("URL malformada: {url}");
    };

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", server.puerto())).unwrap();
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
        .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    assert!(
        head.contains("200 OK"),
        "el MediaServer no devolvió 200 para {}: {head}",
        png.display()
    );
    assert!(
        head.contains("Content-Type: image/png"),
        "Content-Type inesperado: {head}"
    );
    let body = &raw[split + 4..];
    // Cabecera PNG: 89 50 4E 47.
    assert_eq!(&body[..4], &[0x89, 0x50, 0x4E, 0x47], "no es un PNG válido");
}
