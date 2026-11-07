# Scanner drivers the Trindle image should carry

Scan finds scanners two ways: over the network (eSCL, also called AirScan) and over USB through SANE. A scanner works only if the image already carries its driver, so the image should include all of these (Debian 13 package names, all free software in main):

- `libsane1` and `libsane-common`: the SANE backends for hundreds of models (Epson, Canon Pixma, Fujitsu, Plustek, Genesys, Avision and many more) and the rules that let a person use a USB scanner.
- `ipp-usb`: shares a USB printer-scanner (such as the Epson ET-2710) as a driverless network scanner on this computer. Most printers made since about 2015 work this way with no vendor driver.
- `sane-airscan`: SANE's driverless network scanning (eSCL and WSD).
- `hplip` and `libsane-hpaio`: HP printer-scanners.
- `sane-utils`: the `scanimage` test tool, for support.

Vendor drivers that are not in Debian (Epson Image Scan, Brother brscan, Samsung and Xerox unified driver, Canon ScanGear) cover old models only. Their licences must be checked before any of them go on the image.

Seen on the Framework laptop, 26 October 2026: the ET-2710 appears to SANE as "Epson L3150 Series" through the epsonds backend. After one scan it stopped answering until it was switched off and on, so the `ipp-usb` route is the one to prefer.
