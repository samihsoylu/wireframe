# Wireframe modification: avoid Tesseract's unconditional executable TIFF probe.
# This build explicitly disables Leptonica/Tesseract TIFF codecs. These are the
# known probe results; no target test program needs to be compiled or executed.
set(LEPT_TIFF_RESULT 1)
set(LEPT_TIFF_COMPILE_SUCCESS TRUE)
