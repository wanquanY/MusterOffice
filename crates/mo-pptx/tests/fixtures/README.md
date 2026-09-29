# Native timing fixtures

`wps-by-only-timing.xml` is the exact timing subtree of a self-authored six-rectangle calibration deck, edited, saved and reopened in WPS 12.1.22553 on macOS. The enclosing slide only supplies the PresentationML namespace. No user document, third-party artwork, font, or private metadata is included.

The source PPTX SHA-256 is `9923d694f4316d796b5fa858d80b10ab1f820f973aafba95db46fb133fd51620`; fixture SHA-256 is `b9cb3e6bb10752652dcf81d75c9c15f55850bdc8fe46bc3be4e0c2c92ca6b53d`. The source package and native screenshots are retained as local verification materials. See [WPS calibration](../../../../docs/implementation/wps-timing-roundtrip.md).

This fixture exercises actual native omission of container durations, restart defaults, intrinsic scale attribute names, parent begin references and main-sequence navigation lists. Expected intermediate frames are kernel computations; only the final doubled rectangle sizes were independently observed in WPS. It is not evidence of complete Office/WPS compatibility.
