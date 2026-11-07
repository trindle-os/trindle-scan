# Changelog

## 0.1.0

First version, still being finished.

- The toolkit comes from its GitHub repository at a fixed commit, so a clean clone builds. The program is split into files by job.
- Settings, profiles and the page keep folder are written whole and put in place in one step, so a crash cannot leave a half-written file. Home folder lookup and the word-file list come from the shared trindle-common crate. With no usable home folder the program says so in words and writes nothing, instead of writing into the current folder.
- Finds network scanners (eSCL) and USB scanners (SANE), with an address entered by hand as the way round other network segments.
- Profiles, one-click scan, feeder and two-sided scanning, manual two-sided, batch scanning and blank-page skipping.
- Page list with rotate, flip, crop, delete, undo and redo, and a large view with zoom, pan and a crop box.
- Save as PDF, JPEG, PNG or multi-page TIFF; PDF details, password and splitting; auto-save into a folder.
- Searchable PDF by text recognition.
- The program looks for scanners as soon as it opens, and shows the scanner list first when none is remembered; a lone scanner is chosen for you.
- Laptop cameras are no longer listed as scanners; USB printers shared by ipp-usb are.
- A USB scanner is left alone for five seconds after a scan. The Epson ET-2710 on USB stopped answering after one or two scans until switched off and on; with the rest, four scans in a row worked (one run, 27 October 2026).
- Black and white scans are written into PDF files as one-bit pictures, so the files are small.
- The PDF password box hides what is typed, and the PDF uses AES-256.
- Pages are also kept in a private recovery folder while the program is open; the README says where and for how long.
- The toolkit now draws windows with the graphics chip when it can, so the program links libwayland-client and libffi as well as libxkbcommon; the plugin does not carry them, because Trindle installs all three.
- Text recognition ignores relative folders on the search path and keeps text that is not valid UTF-8.
