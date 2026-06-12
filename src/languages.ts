// Idiomas ofrecidos para origen (lo usa whisper al transcribir, ADR-004) y
// destino. El código es el ISO que espera whisper.cpp.
export interface Language {
  code: string;
  label: string;
}

export const LANGUAGES: Language[] = [
  { code: "es", label: "Español" },
  { code: "en", label: "Inglés" },
  { code: "pt", label: "Portugués" },
  { code: "fr", label: "Francés" },
  { code: "de", label: "Alemán" },
  { code: "it", label: "Italiano" },
  { code: "ja", label: "Japonés" },
  { code: "zh", label: "Chino" },
];
