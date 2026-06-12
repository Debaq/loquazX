fn main() {
    // espeak-ng (vía piper-rs -> espeak-rs-sys) se compila en Linux con soporte de
    // libpcaudio, pero su build.rs no emite el enlace a la librería, dejando los
    // símbolos `audio_object_*` sin resolver al linkear el binario. La enlazamos
    // aquí (ADR-009). pcaudiolib es una dependencia de sistema, igual que ffmpeg.
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=pcaudio");
        // espeak-ng también usa libsonic (cambio de velocidad/tono) sin enlazarla.
        println!("cargo:rustc-link-lib=dylib=sonic");
    }

    tauri_build::build()
}
