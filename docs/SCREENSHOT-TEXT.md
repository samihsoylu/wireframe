# Extract text from a screenshot

Choose **File → Extract text from screenshot…**, or select an image and choose **Extract text…** in its inspector. Text extraction recognizes the original image on your device and opens a text preview. Finish any active text or notes edit first; another extraction waits for prior processing to finish.
Review and correct the result, then choose **Copy text**. **Cancel**, **Close** or
Escape dismisses the preview. Recognition, corrections in the preview and
copying do not add controls, import assets, change image captions or create an
Undo step. An existing Redo operation remains available.

English printed text works best. Recognition may miss small, blurred, rotated or
handwritten text, unusual layouts and text in other languages. A blank image
shows **No text found**. Use PNG, JPEG or GIF images up to 32 MiB and 16
megapixels; only the first GIF frame is recognized. Transparent areas are treated
as white. OCR operates on the supplied source image, not a screen capture of
Wireframe or a filtered canvas rendering.

The backend is real local Tesseract 5.5.4 recognition with the official English
fast model. Engine libraries are linked statically and the model is embedded in
the executable. It does not need an OCR installation, account, API key, network,
temporary screenshot file or a downloaded model cache. The UI uses one background
worker and requests cancellation when dismissed. One application-wide permit remains held until processing finishes, including after closing a preview. Another extraction waits for that worker to finish. The engine checks its30-second deadline and cancellation during text recognition; image decoding, model initialization and page segmentation can finish before those checks. The deadline is not a guaranteed total wall-clock limit. No partial text is copied automatically.

See [backend build, limits, source and model licensing](../vendor/wireframe-ocr/README.md).

## Integration and acceptance

The reusable backend lives in `vendor/wireframe-ocr`; the transient dialog lives
in `src/app/screenshot_text.rs`. The parent integration includes selected-image and file screenshot entrypoints, modal/shortcut blocking and cancellation on session reset/exit.
This isolated delivery does not establish native UI or all-platform acceptance.
Its current evidence and unresolved integration checks are in the owning
`delivery/screenshot-text-local/CURRENT.md` checkpoint outside the source tree.
