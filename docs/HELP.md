# Trindle Scan help

This page describes Trindle Scan version 0.1.0: what it does, how to start it, every setting and what it changes, what can go wrong and how to put it right, and what the program can and cannot see.

## What Trindle Scan does

You put paper on a scanner and press Scan. The pages appear in a list. You tidy them, then save them as a PDF, JPEG, PNG or TIFF file. It can also save into a folder by itself as each scan finishes.

It finds two kinds of scanner:

- **Network scanners** that need no driver (the kind called eSCL or AirScan). Most printers with a scanner made in the last ten years or so can do this.
- **USB scanners** that the computer's scanner software (SANE) knows about.

Each scanner is listed once, even if it can be reached both ways.

## Starting it

Trindle Scan is not there by default. Install it from Apps in Trindle, then open it from Apps.

The first time you install it, it asks for three permissions: your home network (to find scanners), your files (to save into) and USB devices (for USB scanners). It asks for nothing else.

When the program opens, it looks for scanners for about twelve seconds. It does not look before you open it, and it only looks on your own network and on USB. If the scanner list is showing when the search ends and it found exactly one scanner, it goes straight to the main window.

The first time you press Scan with no profile, it asks you to make a profile first, and starts the scan as soon as the profile is saved.

## The main window

- **The bar** at the top has the Scan button, the Save button, and buttons for rotate left, rotate right, flip, look at a page, undo, redo and delete.
- **The page list** shows a thumbnail of every page, numbered. A page that has been saved is marked "saved" in its caption.
- **The note under the bar** says what just happened, such as "Scanned 3 pages" or where a file was saved. Problems show there in red words.
- **The status line** shows the number of pages, how many are selected, and the state of the scanner ("Looking for scanners", "Asking ...", "... is ready", "Scanning page 2").
- **Settings** and **Shortcuts** buttons open their own screens. **Add pictures** adds picture files to the list. A **Thumbnails** slider changes the size of the thumbnails. **Open folder** appears after a save and opens the folder the file went into. **Clear saved** appears once some pages are saved and takes them out of the list.

### Selecting pages

Click a page to select only that one. Ctrl and click adds or removes a page. Shift and click selects from the last selected page to the one you click. Ctrl+A selects every page.

Rotate, flip, delete, move and crop work on the selected pages. Save works on the selected pages, or on every page when none is selected.

### Undo

Undo and redo step back and forward through your changes to the page list (deleting, rotating, flipping, cropping, moving, adding). The program remembers the last 30 steps.

### Looking at a page

The Look button opens the large view for the first selected page.

- The mouse wheel zooms, and so do the plus and minus keys. The 0 key shows the whole page. The buttons also give Zoom in, Zoom out, Whole page and Actual size.
- Drag the page to move it.
- Previous page and Next page step through the list.
- **Crop** has four sliders, Left, Top, Right and Bottom, each a cut as a share of the page. The crop shape can be Free, Square, 4 to 3, 3 to 2 or 16 to 9. Apply crop applies it to every selected page. Remove crop undoes it.

## Scanners

Press **Choose a scanner** (or the scanner's name) to see the list. Each entry shows the scanner's name, with its address and port for a network scanner, or "USB" for a USB one.

- **Search again** looks again.
- **Choose another scanner** goes back to the list.
- **Enter an address by hand** is for a scanner on another network segment, which is not found by itself. Type its address, for example 192.168.1.40, with a port after a colon if it is not 80, then press Add. The program asks the scanner what it can do before it keeps it. The address must be on your own local network: the program refuses anything else.
- When you choose a scanner, the current profile remembers it. Next time, the program looks for that scanner again by name, and by address if the name has gone.

A scanner plugged into USB after the program opened is found when the program is opened again.

## Profiles

A profile is a named set of scan settings. Press **New** to make one. You can start from one of three ready-made profiles and change it later:

- **Document, black and white:** A4, 300 dots per inch, black and white, PDF.
- **Document, colour:** A4, 300 dots per inch, colour, PDF.
- **Photo:** the largest size the scanner can do, 600 dots per inch, colour, JPEG.

The profile list lets you **Edit**, **Move up**, **Move down**, **Duplicate** (the copy is named "... copy"), **Make default** and **Delete this profile**. The first profile you make is the default. The big Scan button uses the current profile. A profile needs a name, and no two profiles may share a name.

### Test scan

In the profile editor, **Test scan** scans one page with the settings as they are and shows it large. It is not added to your pages. Use it to try a resolution or brightness before a real scan.

### Every profile setting

The program only offers choices that your scanner says it can do. If you choose something the scanner cannot do, the program uses the nearest thing it can and tells you in the note under the bar.

| Setting | What it changes |
|---|---|
| Name | The profile's name. It also fills the {profile} part of file names. |
| Source | Glass; Feeder; Feeder, both sides (only if the scanner can do both sides in one pass); Feeder, turn the stack over (fronts first, then you turn the stack over and press Scan for the backs, and the pages are put in order). |
| Page size | A4, A5, A3, Letter, Legal, or the largest the scanner can do. If the size is bigger than the scanner can manage, the largest is used. If the scanner does not say how big its glass is, A4 is assumed. |
| Resolution | Dots per inch, from 75 to 1200 as far as the scanner offers. Higher gives bigger files and slower scans. |
| Colour | Colour, Grey, or Black and white. |
| How dark counts as black | Only for Black and white. A number from 0 to 100, 50 to start. |
| Brightness and contrast | From -100 to 100, 0 to start. Reset brightness and contrast puts both back to 0. |
| Turn every page | No, or a fixed turn of 90, 180 or 270 degrees. |
| Straighten pages | Levels pages that came in slightly crooked. |
| Trim borders and extra margin | Removes dark scanner borders, then trims the empty margin and leaves a small even one. |
| Skip blank pages | A page with almost no marks is left out as it arrives. The only page of a scan is never skipped. |
| Ask for another page after each scan | Glass only. After each page, asks "Another page?" so you can put the next one down. |
| Batch | One scan; Several runs; Until I stop. |
| Number of runs | For Several runs: 2, 3, 4, 5, 10 or 20. |
| Pause between runs | Wait for me, or 5, 10, 30, 60 or 120 seconds. While the pause counts down you can press Skip pause, or Stop to end the batch. With "Wait for me" you press Continue when the next stack is in. |
| All runs in one file | Keeps the pages of all the runs together when saving automatically. Off means each run is saved on its own. |
| Save automatically | Saves into the folder below as soon as a scan finishes. It does nothing until a folder is chosen. |
| Make text searchable | PDFs get invisible text so you can search and select it. See "Searchable PDFs" below. |
| Folder | Where automatic saves go. |
| File type | PDF, JPEG, PNG or TIFF. This is also the type used when you press Save yourself. |
| File name | The naming pattern. See below. |
| Clear the pages from the list afterwards | After an automatic save, removes those pages from the list. Off means they stay, marked saved. |

Changes to a profile are saved as you make them.

### File name patterns

The default pattern is "Scan" followed by the date and time. You can use these parts in curly brackets:

`{year}` `{month}` `{day}` `{hour}` `{minute}` `{second}` `{counter:3}` `{profile}`

`{counter:3}` is a number with three digits (001, 002 and so on); you can choose 1 to 9 digits. The program looks in the folder for the highest number already used with that name and goes one higher. A slash in the pattern makes a folder, so `{year}/{month}/Scan {counter:3}` files scans by year and month. The Profile editor shows the next file name as an example.

Characters that cannot go in a file name (such as a colon or a question mark) are replaced with a dash. The program never overwrites an existing file made by an automatic save or a picture save: if the name is taken it adds a number ("Scan 2").

## Scanning

1. Put paper on the glass or in the feeder.
2. Press **Scan**, or press 1 to 9 on the keyboard to scan with the first to ninth profile.
3. Pages appear in the list as they arrive. Press **Cancel** to stop a scan; pages that had already arrived are kept.

On glass you are asked "Another page?" after each page unless you turned that off; press **Scan next** to carry on or **Finish** to stop.

For **Feeder, turn the stack over** the program scans the fronts, tells you to turn the stack over and press Scan for the back sides, then puts the pages in order.

For batches the note shows "Run 1 done." and the pause counts down. When the batch is over the note gives a summary such as "Batch finished: 3 runs, 12 pages."

### Searchable PDFs

With **Make text searchable** on, the program reads the text on each page (a few seconds per page) with the Tesseract program carried in the plugin, and writes it invisibly into the PDF. You choose the language of the text, and a second language if the pages use two. If a page cannot be read, it is saved without text and the note says which page. Reading a single page is given up after three minutes. Press **Cancel** while it reads to cancel the save; the pages stay in the list.

If Tesseract is not on the computer, the setting says so and pages are saved without searchable text.

## Saving

Press **Save**. What happens depends on the file type of the current profile:

- **PDF:** you are asked for a file name. All the pages you are saving go into one PDF. Settings can split a long PDF into several files (below).
- **JPEG, PNG or TIFF:** you are asked for a folder. JPEG and PNG make one file per page ("Scan 1", "Scan 2" and so on; a single page is just "Scan"). TIFF makes one file with every page in it. A JPEG from a scan that was not changed keeps the scanner's own picture.

Where a file chooser from the desktop is not available, such as inside the plugin's box, Trindle Scan shows its own picker with your usual folders (Documents, Downloads, Pictures, Music, Videos and Desktop). It will not let you use a file name that is already taken. It can make a new folder (a plain name, with no slashes and not starting with a dot).

The save window opens in the folder you last saved to, or else the folder set in Settings. After a save the note says where the file went, and **Open folder** opens that folder.

Saved pages are marked "saved". If you turn on "Take saved pages out of the list after saving" in Settings, they leave the list at once. If a save fails the pages stay in the list and the note tells you why.

### Add pictures

**Add pictures** lets you choose JPEG, PNG or TIFF files to add to the page list, where you can tidy them and save them like scanned pages. You can also drop picture files onto the window. Files that cannot be read are counted in the note ("One file could not be read."). Dropping text or anything that is not a picture is refused.

## Settings

Open **Settings** from the main window. Everything saves as you change it. There are no files to edit.

| Setting | Default | What it does |
|---|---|---|
| Folder new profiles save into | none | Copied into each profile you make as its automatic save folder. It is also where the save window opens before you have saved anything. |
| File name new profiles start with | Scan, then the date and time | Copied into each new profile as its naming pattern. |
| Thumbnail size | 150 pixels wide | A slider from 90 to 300. |
| Units for page sizes | Millimetres | Millimetres or inches, wherever page sizes are shown. |
| Take saved pages out of the list after saving | off | Removes pages from the list once they are saved. |
| PDF title | "Scan" if empty | Written into saved PDF files. |
| PDF author, subject, keywords | empty | Written into saved PDF files. |
| Pages in each PDF file | All | All, or 2, 5, 10, 20 or 50. A long scan then becomes several files named "... part 1", "... part 2" and so on. |
| Password to open the PDF | none | The PDF is locked with AES-256 and asks for this password to open. It is kept in the settings file as plain text, so use it only for casual privacy. |
| Folder to keep pages in while the program is open | the user's runtime folder, else the system temporary folder | Where each page is copied as it arrives, so a crash does not lose it. A change is used the next time the program starts. |

Changing a setting here does not change profiles you have already made.

## Keyboard

The keys for scanning and editing work on the main window.

| Keys | What they do |
|---|---|
| 1 to 9 | Scan with the first to ninth profile |
| Ctrl+A | Select every page |
| Delete | Delete the selected pages |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| Alt+Left, Alt+Right | Move the selected pages earlier or later |
| Tab and Shift+Tab | Move between buttons and pages |
| Space or Enter | Press the button or choose the page that has the focus |
| Plus, minus, 0 | In the large view: zoom in, zoom out, whole page |

Every control has a name for screen readers, and scan results and problems are announced.

## When something goes wrong

Problems show as red words under the bar. Nothing ever pops up over the window.

**"No scanner found. Check it is on the same network as this computer, or type its address below."**
The scanner is off, asleep, or on another network. Switch it on, check that it is on the same network (Wi-Fi or cable) as this computer, and press Search again. If it still does not appear, use Enter an address by hand. Some printers need their network scanning (AirScan or eSCL) switched on in their own settings.

**"The scanner did not answer."** followed by a detail
The detail tells you more: it did not answer in time, it answered with an error code, it did not say what it can scan, or it refused the scan. Check the scanner is on and awake and press Try again. If it keeps happening, switch the scanner off and on.

**"The scanner is busy. Trying again, 55 seconds left."**
Another job (or another computer) is using the scanner. The program tries again every five seconds for up to a minute. Press Stop to give up. If it ends with "The scanner is still busy. Try again in a moment.", wait for the other job to finish and press Try again.

**"Put paper in the feeder."**
The feeder is empty. Load it and press Try again, or press **Use glass instead** to scan from the glass this time.

**"Paper jam."**
Clear the jam on the scanner, then press Try again.

**"That address is not on your local network."**
The program only talks to scanners at addresses on your own network. A public address, or a name that points outside your network, is refused on purpose.

**"The scanner would not accept its ... setting, so nothing was scanned."**
The scanner turned down one of the profile's settings. Open the profile and try another source, size or resolution, or use Test scan.

**"The scanner cover is open. Close it and try again."**
Close the lid or cover on the scanner.

**"Check the scanner is plugged in and switched on."** (USB)
Check the cable and the power. A USB scanner plugged in after the program opened is found when the program is opened again.

**"This program is not allowed to use the USB scanner."**
Allow Use scanners and USB devices for Trindle Scan in its permissions, then open it again.

**"USB scanners need scanner software (SANE) that is not on this computer."**
The computer has no SANE scanner software. Network scanners still work.

**"The scanner sent the picture in a form this program cannot read."**
This scanner sends pictures in a way Trindle Scan does not support. Try the scanner's network (AirScan) mode if it has one.

**"One page came back damaged and was skipped."**
That page arrived broken. Scan it again.

**"Could not save: ... The pages are still here."**
The folder may be full, read-only or gone. Nothing was lost: choose another folder and press Save again. Automatic saves give the same advice ("Could not save automatically").

**"Something went wrong inside the program and the job was stopped. Your pages are still here."**
The program stopped a job that failed. Your pages stay in the list; save them, then try again. If it keeps happening, tell us (see below).

**"N pages were left from last time."**
The program closed without saving, for example in a crash. Pages are kept in a private recovery folder. Press **Recover them** to put them back in the list, or **Discard them** to throw them away.

**"Could not keep your settings" or "Could not keep your profiles"**
The settings folder (under ~/.config/trindle-scan) cannot be written. Check the disk is not full or read-only.

**"Your home folder is not set, so your settings and profiles cannot be kept this time."**
The computer did not give the program a home folder. The program works, but nothing is saved between runs.

**"Reading text needs the Tesseract program, which is not on this computer."**
The plugin was built without text reading. Pages are saved without searchable text.

**"The text on page 3 could not be read. That page is saved without searchable text."**
Tesseract could not read that page (it may be blank or a picture). The PDF is still saved.

**Closing the window with unsaved pages**
You are asked "Close without saving?" with "One page is not saved" or "N pages are not saved", and "A scan is still running" if one is. Choose **Save** to save first, **Discard and close**, or **Keep working**.

### Scanner notes

- The Epson ET-2710 on a USB cable appears in the list as "Epson L3150 Series", which is the name the Linux scanner software gives it. It stops answering if it is spoken to in the seconds after a scan, so the program leaves a USB scanner alone for five seconds after each scan. In one test, four scans in a row worked. Longer use on this printer is not proved.
- A better route for that printer is `ipp-usb`, which shares it as a driverless network scanner. Trindle Scan lists such a scanner, but this route has not been tried with that printer.
- No network scanner has been tested on real hardware yet. The program is tested against a simulated scanner.
- If your scanner does not appear, send a bug report with its make and model.

## Privacy: what Trindle Scan can and cannot see

- **Your network.** It sends a question to the local network asking for scanners (multicast DNS, on the standard address 224.0.0.251) and talks to the scanners that answer. It will only connect to addresses on your own local network (private and link-local addresses), never to the internet. Talking to a network scanner is plain, unencrypted HTTP, as most scanners work, so anyone on your network could see a scan in transit.
- **USB.** It can use USB scanners through the computer's scanner software. It cannot see other USB devices.
- **Files.** It reads and writes only what you choose: folders and files you pick in the save or add window, and its own settings, profiles and recovery pages. In its own picker it offers Documents, Downloads, Pictures, Music, Videos and Desktop.
- **Where your scans live.** Pages are held in memory while you work, and each one is also copied to a recovery folder (`trindle-scan-pages`) so a crash does not lose them. The folder is yours alone (mode 0700, files 0600) and is emptied when you close the program normally. If the folder is not a real folder owned by you, the program does not use it. Settings and profiles are plain files in `~/.config/trindle-scan/`, readable only by you.
- **Text reading.** Done on this computer by Tesseract. Nothing is uploaded.
- **The internet.** Trindle Scan does not use the internet. It has no account, no usage reports and no update check.

## Reporting a problem

Report bugs at https://trindle.buildmorphic.com/report, and ask questions on the Buildmorphic community at https://trindle.buildmorphic.com. Say what you did, what you expected and what happened instead, and give the version shown at the top of this page, your scanner's make and model, and whether it is a network or USB scanner. A picture of the window helps. For a security problem, do not post the details in public: see SECURITY.md.
