# Owned delivery integration input

`input.json` is a separate, authored derivative of the repository's original
`native-export/request.json`. It retains two pages, all 15 objects, the master,
layout, native groups, paths, connector and checker image. Only document identity,
title, run text and the explicit default font are changed for this test.

Every run contains `A A`, bound through the explicit manifest to the original
`fixtures/fonts/owned.ttf` (SHA-256
`5e182a1675e0255bce1ff90e6a00d1fee0c12acee868987af1b4f39bf27cabc9`).
This synthetic font deliberately draws A as a triangle; the preview is a
font/resource/format integration probe, not a designed deck or a typography
quality sample. The image bytes are `../native-export/resources.bin`.

The fixture requires no system fonts, external network or private content.
It must pass through the actual writer and native resource-page renderer.
No production text or unsupported object is substituted for test success.
Failure logs from the initial writer-default inheritance defect remain under
the ignored delivery verification directory; the original native-export fixture
and historical evidence are unchanged.
