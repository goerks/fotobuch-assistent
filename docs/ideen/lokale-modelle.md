# Lokale Modelle und Bibliotheken für bessere Auswahl (Recherche, Stand 2026-10-26)

Nur gemerkt, noch nichts umgesetzt. Ziel: Duplikate, Unschärfe, Gesichter und die beste Wahl je Serie verbessern, ohne Cloud. Die App wird später verkauft (einmaliger Kauf): Lizenzen der Gewichte müssen kommerzielle Nutzung erlauben.

## Kandidaten

| Aufgabe | Kandidat | Lizenz | Eignung |
|---|---|---|---|
| Gesichter finden | [YuNet](https://huggingface.co/opencv/face_detection_yunet/blob/main/README.md) | MIT (verifiziert) | Sehr klein und schnell, Gesichter ab etwa 10×10 Pixeln |
| Augen offen, Lächeln | MediaPipe Face Landmarker | Apache 2.0 (nur aus Wissen, nicht verifiziert) | Liefert Blinzeln und Lächeln als Werte, Seite war nicht abrufbar |
| Ähnliche Fotos | [DINOv2](https://huggingface.co/thoddnn/dinov2-base/blob/main/README.md) small/base | Apache 2.0 (verifiziert) | Embeddings, erkennen auch andere Blickwinkel und Zuschnitte |
| Ähnlichkeit, Screenshots/Dokumente erkennen | [CLIP](https://github.com/openai/CLIP) ViT-B/32 oder [SigLIP 2](https://telemetry.superlinked.com/models/google-siglip2-base-patch16-224.md) | CLIP-Code MIT, SigLIP 2 Apache 2.0 (verifiziert); Gewichte von CLIP nicht verifiziert | SigLIP 2 base etwa 375 Mio. Parameter, für schwache PCs schwer |
| Ästhetik | [LAION Aesthetic Predictor](https://github.com/christophschuhmann/improved-aesthetic-predictor) | Repository Apache 2.0, Gewichte ungeklärt | Braucht CLIP ViT-L/14 (gross), bekannte Verzerrungen |
| Technische Qualität | [pyiqa](https://github.com/chaofengc/IQA-PyTorch) (MUSIQ, TOPIQ, NIMA) | PolyForm Noncommercial | Nicht nutzbar für ein verkauftes Produkt |

Meiden oder erst genau prüfen: MobileCLIP (Gewichte unter «Apple ML Research Model TOU», vermutlich nur Forschung), InsightFace-Modelle (SCRFD, ArcFace; Gewichte vermutlich nur Forschung).

## Laufzeit
- [tract](https://github.com/sonos/tract): reines Rust, Apache 2.0 oder MIT, ein einzelnes Programm auf Windows, Mac und Linux, aktiv gepflegt. Favorit für kleine Modelle.
- [ort](https://docs.rs/crate/ort/latest) (ONNX Runtime): schneller bei grossen Modellen, aber noch Release Candidate (2.0.0-rc.15) und braucht eine mitgelieferte Bibliothek.
- Modelle fest mitliefern, kein Download zur Laufzeit.

## Beste Wahl pro Serie
Gesamtnote aus Schärfe (im Gesicht besonders), Augen offen, Lächeln, Belichtung, Auflösung und optional Ästhetik. Gewichte später einstellbar.

## Vorschlag in Stufen
1. Ohne Modell: Schärfe im Gesichts- oder Bildmittelpunkt, Belichtung, Rauschen.
2. Gesichter und Augen (YuNet plus Landmarken).
3. Ähnlichkeit per Embedding (DINOv2-small oder CLIP B/32), Screenshots/Dokumente erkennen.
4. Ästhetik, erst nach Klärung von Lizenz und Grösse der Gewichte.

## Nicht verifiziert
Dateigrössen (grobe Annahme: YuNet unter 1 MB, DINOv2-small 20–90 MB, CLIP B/32 90–350 MB) und die Lizenzen der Gewichte von MediaPipe, OpenAI-CLIP und OpenCLIP.

## Offene Fragen
Wie gross darf die Installation werden? Sollen die Kriterien der Auswahl einstellbar sein?
