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
}
