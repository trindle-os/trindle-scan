# Words of Trindle Scan: every word the program shows or speaks, in English.

## Window and general (src/main.rs)

# Name of the program, shown as the window title.
scan-app-name = Trindle Scan

# Red words at start-up when the home folder is not set to a full path, so settings and profiles cannot be kept.
scan-main-error-no-home = Trindle Scan can't find your home folder, so it can't save your settings and profiles this time.

# Red words when the program's own settings could not be saved. The reason from the computer follows.
scan-main-error-keep-settings = Trindle Scan couldn't save your settings: { $why }

# Red words when the profiles could not be saved. The reason from the computer follows.
scan-main-error-keep-profiles = Trindle Scan couldn't save your profiles: { $why }

# Red words under the bar when Scan is pressed with no scanner chosen.
scan-main-error-no-scanner = You haven't chosen a scanner yet.

# Red words when Scan is pressed before the scanner has said what it can do.
scan-main-error-wait-caps = Trindle Scan is still checking what the scanner can do. Try again in a moment.

# Red words on the profile editor when Test scan is pressed with no scanner.
scan-main-test-choose-scanner = Choose a scanner first, then try again.

# Red words on the profile editor when the test scan failed. The reason follows.
scan-main-test-failed = The test scan did not work: { $why }

# Red words on the profile editor when the scanner sent no page for the test scan.
scan-main-test-no-page = The scanner didn't send a page. Put a page in and try again.

# Spoken by the screen reader when the test scan has finished.
scan-main-announce-test-done = Test scan finished

# Spoken by the screen reader when the test scan did not work.
scan-main-announce-test-failed = Test scan did not work

# Note under the bar when a page came back damaged and was left out.
scan-main-note-damaged = One page was damaged, so Trindle Scan skipped it.
scan-save-page-unreadable = Trindle Scan couldn't read a scanned page, so nothing was saved.
scan-save-page-encode = Trindle Scan couldn't turn a page into a picture: { $why }

# Red words when a scan stopped part way. The first part says what went wrong, then how many pages were kept.
scan-main-error-with-kept =
    { $words } { $n ->
        [one] The page that did arrive is still here.
       *[other] The { $n } pages that did arrive are still here.
    }

# Note after saving a long scan as several PDF files: the folder and file name, then how many files.
scan-main-save-files =
    { $path } ({ $n ->
        [one] { $n } file
       *[other] { $n } files
    })

# Note under the bar after pages were saved. The place follows.
scan-main-note-saved = Saved to { $path }

# Note under the bar after an automatic save at the end of a batch: the batch summary, then where it was saved.
scan-main-note-saved-summary = { $summary } Saved to { $path }

# Red words when pages could not be saved. The reason follows.
scan-main-error-save = Trindle Scan couldn't save your scans: { $why }. Your pages are still here.

# Red words when an automatic save did not work. The reason follows.
scan-main-error-autosave = Trindle Scan couldn't save your scans automatically: { $why }. Your pages are still here.
scan-main-error-crashed = Something went wrong, so Trindle Scan stopped. Your pages are still here.

# Note under the bar after pictures were added from files.
scan-main-note-added =
    Added { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    }

# Note under the bar after pictures were added and some files could not be read.
scan-main-note-added-failed =
    Added { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    }. { $failed ->
        [one] Trindle Scan couldn't read one file.
       *[other] Trindle Scan couldn't read { $failed } files.
    }

# Red words when the folder of saved files could not be opened.
scan-main-error-open-folder = Trindle Scan couldn't open the folder.

# Red words when pages could not be copied to the recovery folder, so a crash would lose them.
scan-main-error-not-kept =
    { $n ->
        [one] Trindle Scan couldn't copy one page to the recovery folder. If Trindle Scan crashes, you'll lose it.
       *[other] Trindle Scan couldn't copy { $n } pages to the recovery folder. If Trindle Scan crashes, you'll lose them.
    }

# Note under the bar after pages from last time were brought back.
scan-main-note-recovered =
    Recovered { $n ->
        [one] { $n } page
       *[other] { $n } pages
    } from last time.

# Note when the front sides are done and the person must turn the stack over.
scan-main-note-turn-over = Turn the stack over, then press Scan to scan the backs.

# Note and question on the bar after a glass scan, asking whether to scan another page.
scan-main-note-another-page = Scan another page?

# Note under the bar when one run of a batch has finished.
scan-main-note-run-done = Scan { $n } is finished.

# Summary when a batch has finished: how many runs and how many pages.
scan-main-summary-batch =
    Batch finished: { $runs ->
        [one] { $runs } scan
       *[other] { $runs } scans
    }, { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }.

# Red words when the scanner stayed busy for the whole waiting time.
scan-main-error-busy-still = The scanner is still busy. Try again in a moment.

# Red words while the program waits for a busy scanner, counting down.
scan-main-error-busy-retry =
    The scanner is busy. Trindle Scan will try again for another { $n ->
        [one] { $n } second
       *[other] { $n } seconds
    }.

# Note under the bar when a scan has finished.
scan-main-note-scanned =
    Scanned { $n ->
        [one] { $n } page
       *[other] { $n } pages
    }

# Note under the bar when a scan has finished and some blank pages were left out.
scan-main-note-scanned-skipped =
    Scanned { $n ->
        [one] { $n } page
       *[other] { $n } pages
    } and skipped { $blanks ->
        [one] { $blanks } blank page
       *[other] { $blanks } blank pages
    }

# File name for one part of a PDF that was split into several files.
scan-main-part-name = { $stem } part { $n }

# Red words when retrying was stopped because the scanner was busy.
scan-main-error-stopped-busy = Scanning stopped because the scanner was busy.

# Words while an address typed by hand is being checked.
scan-main-note-checking-address = Checking the address

# Name of the list of picture files in the file chooser.
scan-main-filter-pictures = Pictures

# Red words when something dropped on the window gave no files. The reason follows.
scan-main-drop-nothing = Trindle Scan couldn't add the files: { $why }

# Red words when text, not a file, is dropped on the window.
scan-main-drop-text = You can only drop picture files here.

## Status line (src/main.rs)

# Status line: how many pages are in the list.
scan-status-pages =
    { $n ->
        [one] { $n } page
       *[other] { $n } pages
    }

# Status line: how many pages are in the list and how many are selected.
scan-status-pages-selected =
    { $n ->
        [one] { $n } page
       *[other] { $n } pages
    }, { $sel } selected

# Status line and bar label while a page is being scanned.
scan-status-scanning = Scanning page { $n }

# Status line and scanner screen words while the network is searched.
scan-status-searching = Looking for scanners

# Status line when the chosen scanner has said what it can do.
scan-status-ready = { $name } is ready

# Status line while the chosen scanner is being asked what it can do.
scan-status-asking = Contacting { $name }

# Status line when no scanner is chosen.
scan-status-none = No scanner chosen

# Status line: the page count, then the scanner state.
scan-status-line = { $pages }      { $state }

# Red words under the profile name box when it is empty.
scan-profiles-error-name-empty = Give the profile a name.

# Red words under the profile name box when another profile has that name.
scan-profiles-error-name-taken = Another profile already has that name.

# Red words when a page cannot be opened in the large view.
scan-view-error-unshown = Trindle Scan can't show that page.

## Large view of one page (src/main.rs)

# Title above the large view of a page, and its spoken name.
scan-view-page-of = Page { $i } of { $n }

# Heading of the crop part beside the large view.
scan-view-crop-heading = Crop

# Note under the crop heading.
scan-view-crop-note = Set how much to cut off each edge. This changes every selected page.

# Label of the crop slider for the left edge.
scan-view-crop-left = Left

# Label of the crop slider for the top edge.
scan-view-crop-top = Top

# Label of the crop slider for the right edge.
scan-view-crop-right = Right

# Label of the crop slider for the bottom edge.
scan-view-crop-bottom = Bottom

# Spoken name of the left crop slider.
scan-view-crop-left-spoken = Cut from the left

# Spoken name of the top crop slider.
scan-view-crop-top-spoken = Cut from the top

# Spoken name of the right crop slider.
scan-view-crop-right-spoken = Cut from the right

# Spoken name of the bottom crop slider.
scan-view-crop-bottom-spoken = Cut from the bottom

# Label above a crop slider: the edge and the share cut.
scan-view-crop-value = { $name }: { $value }

# Button that takes the crop away.
scan-view-crop-remove = Remove crop

# Button that cuts the selected pages to the crop.
scan-view-crop-apply = Apply crop

# Tip and spoken name of the button that shows the page before.
scan-view-previous = Previous page

# Tip and spoken name of the button that shows the next page.
scan-view-next = Next page

# Button that goes back to the page list from another screen.
scan-common-back = Back

# Button that closes a settings screen.
scan-common-done = Done

# Button that stops what is going on.
scan-common-stop = Stop

# Button that cancels the scan or the picker.
scan-common-cancel = Cancel

# Button that opens a folder picker.
scan-common-choose = Choose

# Button that uses a profile or a starting point.
scan-common-use = Use

# Note that changes are kept as they are made.
scan-common-changes-save = Your changes are saved as you go.

## Bar above the pages (src/main.rs)

# Words on the bar between batch runs when the person must load the next stack.
scan-bar-pause-empty = Put in the next stack, then press Continue.

# Words on the bar counting down to the next batch run.
scan-bar-pause-wait =
    Next scan in { $n ->
        [one] { $n } second
       *[other] { $n } seconds
    }

# Button that carries on to the next batch run.
scan-bar-continue = Continue

# Button that skips the wait before the next batch run.
scan-bar-skip-pause = Skip pause

# Button that scans the next page.
scan-bar-scan-next = Scan next

# Button that finishes a scan session.
scan-bar-finish = Finish

# Main button that starts a scan.
scan-bar-scan = Scan

# Button words when there is no profile yet.
scan-bar-no-profile = No profile yet

# Button words when no scanner is chosen; opens the scanner screen.
scan-bar-choose-scanner = Choose a scanner

# Main button that saves the pages.
scan-bar-save = Save

# Tip and spoken name of the button that turns selected pages left.
scan-bar-rotate-left = Rotate left

# Tip and spoken name of the button that turns selected pages right.
scan-bar-rotate-right = Rotate right

# Tip and spoken name of the button that flips selected pages.
scan-bar-flip = Flip

# Tip and spoken name of the button that opens the large view.
scan-bar-look = View first selected page

# Tip and spoken name of the undo button.
scan-bar-undo = Undo

# Tip and spoken name of the redo button.
scan-bar-redo = Redo

# Tip and spoken name of the button that deletes selected pages.
scan-bar-delete = Delete selected pages

# Button that tries the scan again after a problem.
scan-bar-try-again = Try again

# Button that scans from the glass when the feeder failed.
scan-bar-use-glass = Use the glass instead

# Button that searches the network for scanners again.
scan-bar-search-again = Search again

# Button that opens the scanner screen after the scanner did not answer.
scan-bar-choose-another = Choose another scanner

## Page list (src/main.rs)

# Large words in the empty page list.
scan-pages-empty = Press Scan

# Spoken name of a page tile.
scan-pages-tile-name = Page { $i } of { $n }

# Spoken name of a selected page tile.
scan-pages-tile-name-selected = Page { $i } of { $n }, selected

# Caption under a page tile.
scan-pages-caption = Page { $i }

# Caption under a page tile that has been saved.
scan-pages-caption-saved = Page { $i }, saved

# Words on the bottom bar when pages were left from the last run.
scan-pages-recover-left =
    { $n ->
        [one] { $n } page was left from last time.
       *[other] { $n } pages were left from last time.
    }

# Button that brings back the pages left from the last run.
scan-pages-recover = Recover pages

# Button that throws away the pages left from the last run.
scan-pages-discard = Discard pages

# Label of the thumbnail size slider on the bottom bar.
scan-pages-thumbnails = Thumbnails

# Spoken name of the thumbnail size slider.
scan-settings-thumb-spoken = Thumbnail size, small to large

# Bottom bar button that opens the settings screen.
scan-pages-settings = Settings

# Bottom bar button that opens the shortcuts screen.
scan-pages-shortcuts = Shortcuts

# Bottom bar button that opens the picture picker.
scan-pages-add-pictures = Add pictures

# Bottom bar button that opens the folder pages were saved in.
scan-pages-open-folder = Open folder

# Bottom bar button that removes saved pages from the list.
scan-pages-clear-saved = Clear saved pages

## Folder and picture picker (src/main.rs, src/browse.rs)

# Title of the picker (and of the system file chooser) for saving a PDF.
scan-browse-title-save-pdf = Save scan

# Title of the picker for choosing where pictures are saved.
scan-browse-title-save-folder = Folder for the pictures

# Title of the picker (and of the system file chooser) for adding pictures.
scan-browse-title-pictures = Add pictures

# Title of the picker for the folder a profile saves into.
scan-browse-title-profile-folder = Folder to save scans in

# Title of the picker for the folder new profiles save into.
scan-browse-title-settings-folder = Default folder for new profiles

# Title of the picker for the folder pages are kept in while the program is open.
scan-browse-title-temp-folder = Temporary pages folder

# File name that scans start with, without the ending.
scan-browse-default-name = Scan

# Red words in the picker when a new folder is asked for before a folder is open.
scan-browse-error-open-first-new = Open one of your folders first, then make the new folder inside it.

# Red words in the picker when a new folder name is not plain.
scan-browse-error-folder-name = Folder names can't contain slashes or start with a dot.

# Red words in the picker when a folder could not be made. The reason follows.
scan-browse-error-make-folder = Trindle Scan couldn't create the folder: { $why }

# Red words in the picker when Add is pressed with no pictures chosen.
scan-browse-error-no-pictures = Select the pictures you want, then add them.

# Red words in the picker when Save is pressed before a folder is open.
scan-browse-error-open-first = Open one of your folders first.

# Red words in the picker when the file name is not plain.
scan-browse-error-file-name = File names can't contain slashes or start with a dot.

# Red words in the picker when a file with that name already exists.
scan-browse-error-name-taken = A file with that name is already there. Choose another name.

# Words above the list in the picker when no folder is open yet.
scan-browse-top = Choose one of your folders

# Button in the picker that goes up one folder.
scan-browse-up = Up

# Detail under a folder name in the picker.
scan-browse-kind-folder = Folder

# Detail under a picture that has been chosen in the picker.
scan-browse-kind-chosen = Selected

# Detail under a picture in the picker.
scan-browse-kind-picture = Picture

# Note in the picker when a folder has no folders or pictures.
scan-browse-empty-pictures = No folders or pictures here.

# Note in the picker when a folder has no folders.
scan-browse-empty-folders = No folders here yet.

# Label above the new folder box in the picker.
scan-browse-new-folder = New folder

# Spoken name of the new folder box in the picker.
scan-browse-new-folder-spoken = Name of the new folder

# Button in the picker that makes the new folder.
scan-browse-make-folder = Make folder

# Label and spoken name of the file name box in the picker.
scan-browse-file-name = File name

# Main button of the picker when saving a PDF.
scan-browse-go-save = Save here

# Main button of the picker when adding pictures.
scan-browse-go-pictures =
    Add { $n ->
        [one] { $n } picture
       *[other] { $n } pictures
    }

# Main button of the picker when choosing a folder.
scan-browse-go-folder = Use this folder

## Shortcuts screen (src/main.rs)

# Title of the shortcuts screen.
scan-shortcuts-title = Keyboard shortcuts

# Keys that scan with a profile.
scan-shortcuts-profile-keys = 1 to 9

# What the profile keys do.
scan-shortcuts-profile-what = Scan with the profile at that position in the list

# Keys that select every page.
scan-shortcuts-select-keys = Ctrl+A

# What the select key does.
scan-shortcuts-select-what = Select every page

# Key that deletes the selected pages.
scan-shortcuts-delete-keys = Delete

# What the delete key does.
scan-shortcuts-delete-what = Delete the selected pages

# Keys that undo.
scan-shortcuts-undo-keys = Ctrl+Z

# What the undo keys do.
scan-shortcuts-undo-what = Undo

# Keys that redo.
scan-shortcuts-redo-keys = Ctrl+Shift+Z

# What the redo keys do.
scan-shortcuts-redo-what = Redo

# Keys that move the selected pages earlier.
scan-shortcuts-earlier-keys = Alt+Left

# What the move earlier keys do.
scan-shortcuts-earlier-what = Move the selected pages earlier

# Keys that move the selected pages later.
scan-shortcuts-later-keys = Alt+Right

# What the move later keys do.
scan-shortcuts-later-what = Move the selected pages later

# Keys that move between buttons and pages.
scan-shortcuts-tab-keys = Tab and Shift+Tab

# What the tab keys do.
scan-shortcuts-tab-what = Move between buttons and pages

# Keys that press the focused button or choose the focused page.
scan-shortcuts-press-keys = Space or Enter

# What the space and enter keys do.
scan-shortcuts-press-what = Press the highlighted button or choose the highlighted page

# Note under the list of shortcuts.
scan-shortcuts-note = The keys for scanning and editing pages work on the page list screen.

## Settings screen (src/main.rs, src/settings.rs)

# Title of the settings screen.
scan-settings-title = Settings

# Label above the folder new profiles save into.
scan-settings-folder = Default folder for new profiles

# Words when no folder has been chosen.
scan-settings-no-folder = No folder chosen

# Label above the default file name box.
scan-settings-pattern = Default file name for new profiles

# Spoken name of the default file name box.
scan-settings-pattern-spoken = Default file name pattern

# Label above the thumbnail size slider.
scan-settings-thumb =
    Thumbnail size: { $n ->
        [one] { $n } pixel wide
       *[other] { $n } pixels wide
    }

# Label above the choice of units for page sizes.
scan-settings-units = Units for page sizes

# Spoken name of the units choice.
scan-settings-units-spoken = Units

# Choice of units: millimetres.
scan-settings-units-mm = Millimetres

# Choice of units: inches.
scan-settings-units-in = Inches

# Note showing a page size written in the chosen units.
scan-settings-example = For example, { $size }

# Label of the switch that takes saved pages out of the list.
scan-settings-remove-saved = Remove pages from the list once saved

# Spoken name of the remove saved pages switch.
scan-settings-remove-saved-spoken = Remove pages after saving

# Heading above the PDF details.
scan-settings-pdf-heading = Saved PDF files

# Label above the PDF title box.
scan-settings-pdf-title = Title

# Spoken name of the PDF title box.
scan-settings-pdf-title-spoken = PDF title

# Label above the PDF author box.
scan-settings-pdf-author = Author

# Spoken name of the PDF author box.
scan-settings-pdf-author-spoken = PDF author

# Label above the PDF subject box.
scan-settings-pdf-subject = Subject

# Spoken name of the PDF subject box.
scan-settings-pdf-subject-spoken = PDF subject

# Label above the PDF keywords box.
scan-settings-pdf-keywords = Keywords

# Spoken name of the PDF keywords box.
scan-settings-pdf-keywords-spoken = PDF keywords

# Label above the choice of how many pages go in each PDF.
scan-settings-pdf-split = Pages in each file

# Choice: all pages in one PDF.
scan-settings-pdf-split-all = All

# Spoken name of the pages in each PDF choice.
scan-settings-pdf-split-spoken = Pages in each PDF file

# Label above the PDF password box.
scan-settings-pdf-password = Password to open the PDF

# Spoken name of the PDF password box.
scan-settings-pdf-password-spoken = PDF password

# Note under the PDF password box.
scan-settings-pdf-password-note = Leave this empty for no password. Anyone who can open your settings file can read the password, because it is stored as plain text.

# Title written in a saved PDF when the person has not typed one.
scan-settings-pdf-default-title = Scan

# Heading above the place pages are kept while open.
scan-settings-progress-heading = Temporary pages

# Label above the folder pages are kept in while the program is open.
scan-settings-temp = Temporary pages folder

# Words when the default place is used to keep pages.
scan-settings-temp-default = Default folder

# Note under the folder for pages in progress.
scan-settings-temp-note = This change takes effect the next time you open Trindle Scan.

# A page size with its measurements in millimetres.
scan-settings-size-mm = { $name }, { $w } by { $h } mm

# A page size with its measurements in inches.
scan-settings-size-in = { $name }, { $w } by { $h } in

## Scanner screen (src/main.rs)

# Note when no scanner was found.
scan-scanners-none = No scanner found. Check it is on the same network as this computer, or type its address below.

# Label above the box for a scanner address.
scan-scanners-address = Scanner address

# Spoken name of the scanner address box.
scan-scanners-address-spoken = Scanner address, for example 192.168.1.40

# Button that adds a scanner by its address.
scan-scanners-add = Add

# Note under the scanner address box.
scan-scanners-address-note = Trindle Scan can't find a scanner on another network by itself. Type its address to add it.

## Profiles (src/main.rs, src/profile.rs)

# Title of the screen for making a profile.
scan-presets-title = Make a profile

# Note on the screen for making a profile.
scan-presets-note = A profile stores your scan settings. Choose one to start with. You can change it later.

# Title of the profiles screen.
scan-profiles-title = Profiles

# Button that makes a new profile.
scan-profiles-new = New

# Added after the name of the default profile.
scan-profiles-default-mark = { $name } (default)

# Button that edits a profile.
scan-profiles-edit = Edit

# Tip and spoken name of the button that moves a profile up.
scan-profiles-up = Move up

# Tip and spoken name of the button that moves a profile down.
scan-profiles-down = Move down

# Tip and spoken name of the button that copies a profile.
scan-profiles-duplicate = Duplicate

# Tip and spoken name of the button that makes a profile the default.
scan-profiles-make-default = Make default

# Tip and spoken name of the button that deletes a profile.
scan-profiles-delete = Delete this profile

# Name of a profile that was saved without one.
scan-profile-default-name = Profile

# Name made to keep two profiles from sharing a name: the name, then a number.
scan-profile-numbered-name = { $name } { $n }

# Name of a copied profile.
scan-profile-copy-name = { $name } copy

# File name pattern new profiles start with: the word, then the date and the time. The date and time are filled in when saving.
scan-profile-default-pattern = Scan { $date } { $time }

# Name of the starting point for black and white documents.
scan-profile-preset-bw = Document, black and white

# Name of the starting point for colour documents.
scan-profile-preset-colour = Document, colour

# Name of the starting point for photos.
scan-profile-preset-photo = Photo

# Page size A4.
scan-profile-size-a4 = A4

# Page size A5.
scan-profile-size-a5 = A5

# Page size A3.
scan-profile-size-a3 = A3

# Page size Letter.
scan-profile-size-letter = Letter

# Page size Legal.
scan-profile-size-legal = Legal

# Page size choice: the biggest the scanner can do.
scan-profile-size-largest = Largest supported size

# File type PDF.
scan-profile-format-pdf = PDF

# File type JPEG.
scan-profile-format-jpeg = JPEG

# File type PNG.
scan-profile-format-png = PNG

# File type TIFF.
scan-profile-format-tiff = TIFF

# Note when the scanner did not say how big its glass is.
scan-profile-told-glass = The scanner didn't report its glass size, so Trindle Scan used A4.

# Note when the page size was cut down to what the scanner can do.
scan-profile-told-size = The page size was too big for the scanner, so Trindle Scan used its largest supported size.

# Note when the resolution was changed to one the scanner offers.
scan-profile-told-dpi = The scanner can't do { $dpi } dots per inch, so Trindle Scan used { $near }.

# Note when the scanner has no feeder and the glass was used.
scan-profile-told-feeder = The scanner has no feeder, so Trindle Scan used the glass.

# Note when the scanner cannot scan both sides in one pass.
scan-profile-told-duplex = The scanner can't scan both sides at once.

## Profile editor (src/main.rs)

# Title of the profile editor.
scan-editor-title = Edit profile

# Label above the profile name box.
scan-editor-name = Name

# Spoken name of the profile name box.
scan-editor-name-spoken = Profile name

# Heading above the test scan.
scan-editor-test-heading = Test scan

# Words while the test page is being scanned.
scan-editor-test-busy = Scanning a test page

# Note beside the Test scan button.
scan-editor-test-note = This scans one page with these settings. It isn't added to your pages.

# Note under the test page picture.
scan-editor-test-size = Test page: { $w } by { $h } pixels

# Spoken name of the test page picture.
scan-editor-test-picture = Test scan result

# Label and spoken name of the choice of where pages come from.
scan-editor-source = Source

# Choice of source: the glass.
scan-editor-source-glass = Glass

# Choice of source: the feeder.
scan-editor-source-feeder = Feeder

# Choice of source: the feeder, both sides in one pass.
scan-editor-source-both = Feeder, both sides

# Choice of source: the feeder, with the stack turned over by hand.
scan-editor-source-manual = Feeder, turn the stack over

# Label and spoken name of the page size choice.
scan-editor-size = Page size

# Label above the resolution choice.
scan-editor-dpi = Resolution (dots per inch)

# Spoken name of the resolution choice.
scan-editor-dpi-spoken = Resolution

# Note under the resolution choice when it is high.
scan-editor-dpi-note = A higher resolution gives bigger files and slower scans.

# Label above the colour choice.
scan-editor-colour = Colour

# Spoken name of the colour choice.
scan-editor-colour-mode = Colour mode

# Choice of colour mode: full colour.
scan-editor-colour-colour = Colour

# Choice of colour mode: shades of grey.
scan-editor-colour-grey = Grey

# Choice of colour mode: black and white.
scan-editor-colour-bw = Black and white

# Label above the black and white threshold slider.
scan-editor-threshold = Black and white threshold: { $n }

# Spoken name of the threshold slider.
scan-editor-threshold-spoken = Black and white threshold

# Label above the brightness slider.
scan-editor-brightness = Brightness: { $n }

# Spoken name of the brightness slider.
scan-editor-brightness-spoken = Brightness

# Label above the contrast slider.
scan-editor-contrast = Contrast: { $n }

# Spoken name of the contrast slider.
scan-editor-contrast-spoken = Contrast

# Button that puts brightness and contrast back to zero.
scan-editor-reset-tone = Reset brightness and contrast

# Label above the choice of turning every page.
scan-editor-rotate = Rotate every page

# Choice of turning: do not turn.
scan-editor-rotate-none = None

# Spoken name of the turn choice.
scan-editor-rotate-spoken = Page rotation in degrees

# Label and spoken name of the straighten switch.
scan-editor-straighten = Straighten pages

# Label of the trim borders switch.
scan-editor-trim = Trim borders and margins

# Spoken name of the trim borders switch.
scan-editor-trim-spoken = Trim borders

# Label and spoken name of the skip blank pages switch.
scan-editor-skip-blank = Skip blank pages

# Label of the switch that asks for another page after each scan.
scan-editor-ask-more = Ask for another page after each scan

# Spoken name of the ask for another page switch.
scan-editor-ask-more-spoken = Ask for another page

# Heading above the batch choice.
scan-editor-batch-heading = Batch

# Choice of batch mode: one scan.
scan-editor-batch-one = One scan

# Choice of batch mode: a set number of runs.
scan-editor-batch-several = Several scans

# Choice of batch mode: runs until the person stops.
scan-editor-batch-until = Until I stop

# Spoken name of the batch choice.
scan-editor-batch-spoken = Batch mode

# Label and spoken name of the number of runs choice.
scan-editor-runs = Number of scans

# Label above the pause between runs choice.
scan-editor-pause = Pause between scans (seconds)

# Spoken name of the pause between runs choice.
scan-editor-pause-spoken = Pause between scans

# Choice of pause: wait until the person presses Continue.
scan-editor-pause-wait = Wait for me

# Label and spoken name of the switch that puts all runs in one file.
scan-editor-one-file = All scans in one file

# Heading above what happens after scanning.
scan-editor-after-heading = After scanning

# Label and spoken name of the save automatically switch.
scan-editor-auto-save = Save automatically

# Label above the folder to save into.
scan-editor-folder = Folder

# Label and spoken name of the file type choice.
scan-editor-file-type = File type

# Spoken name of the file name pattern box.
scan-editor-pattern-spoken = File name pattern

# Note showing the name of the next file.
scan-editor-next-file = Next file: { $name }

# Note listing the parts that can be used in a file name. The parts in curly brackets are typed as they are.
scan-editor-parts = You can use these in the file name: { $parts }. Add a / to make a folder.

# Label of the switch that clears the pages from the list after saving.
scan-editor-clear-after = Clear pages from the list after saving

# Spoken name of the clear pages switch.
scan-editor-clear-after-spoken = Clear pages after saving

## Scanner problems (src/escl.rs)

# Red words when the scanner did not answer. The reason follows.
scan-escl-offline = The scanner isn't responding. { $detail }

# Red words when the scanner is busy.
scan-escl-busy = The scanner is busy.

# Red words when the feeder is empty.
scan-escl-no-paper = Put paper in the feeder.

# Red words when paper is stuck.
scan-escl-jam = The paper has jammed.

# Red words when the address is not on the local network.
scan-escl-not-private = That address is not on your local network.

# Red words for any other scanner problem. The reason follows.
scan-escl-other = The scanner had a problem. { $detail }

# Red words when a scan was stopped by the person.
scan-escl-stopped = The scan was stopped.

# Reason given when the scanner's address cannot be found.
scan-escl-detail-no-address = There is no scanner address.

# Reason given when the scanner answered with an error number.
scan-escl-detail-answered = It replied with error { $code }.

# Reason given when the scanner did not say what it can scan.
scan-escl-detail-no-caps = It didn't report what it can scan.

# Reason given when the scanner refused a scan, with its error number.
scan-escl-detail-refused = It couldn't start the scan (error { $code }).

# Reason given when the scanner did not say where the scan job is.
scan-escl-detail-no-job = The scanner didn't send the scan.

# Reason given when the scanner stopped with an error number.
scan-escl-detail-stopped = It stopped with error { $code }.

# Reason given when the scanner's answer could not be read.
scan-escl-detail-broken = It sent a reply Trindle Scan couldn't read.

# Reason: the scanner gave an address with characters this program will not send.
scan-escl-detail-unsafe = It sent an address that Trindle Scan can't safely use.

# Reason: the scanner took far too long to send its answer.
scan-escl-detail-too-slow = It took too long to respond.

## USB scanners (src/sane.rs, src/main.rs)

# Detail under a scanner in the list that is plugged into this computer.
scan-scanners-usb = USB

# Red words when a USB scanner is chosen but the computer lacks the scanner software.
scan-sane-missing = USB scanners need the SANE scanner software, which isn't installed on this computer.

# Red words when the system will not let the program use the USB scanner.
scan-sane-denied = Trindle Scan isn't allowed to use USB scanners. Switch on Use scanners and USB devices in its permissions, then open it again.

# Red words when the scanner lid is open.
scan-sane-cover = The scanner cover is open. Close it and try again.

# Reason given when the USB scanner cannot be reached.
scan-sane-io = Trindle Scan can't reach the scanner. Check it is plugged in and switched on. If you plugged it in after opening Trindle Scan, close Trindle Scan and open it again.

# Red words when the scanner would not take one setting. { $what } is the scanner's own name for it.
scan-sane-refused = The scanner didn't accept the { $what } setting, so nothing was scanned.

# Red words when a USB scanner's name cannot be used.
scan-sane-bad-device = Trindle Scan can't use this scanner.

# Red words when the scanner sends the picture in a form the program cannot read.
scan-sane-frame = The scanner sent the picture in a form Trindle Scan can't read.

## Reading text for searchable PDFs (src/ocr.rs)

# Reason given when the chosen language for reading text is not a plain language code.
scan-ocr-bad-language = Trindle Scan can't read text in that language.

# Reason given when reading the text of one page took too long.
scan-ocr-too-slow = Reading the text on this page took too long.

# Reason given when the text reader stopped with an error on a page.
scan-ocr-failed = Trindle Scan couldn't read the text on this page.

# Words on the bar while the text of the pages is read for a searchable PDF.
scan-ocr-reading = Reading the text on page { $i } of { $n }

# Red words when the person cancelled the reading of the text, which also cancels the save.
scan-ocr-cancelled = Saving was cancelled. The pages are still here.

# Note under the bar when the program that reads text is not on this computer.
scan-ocr-missing = To read text, Trindle Scan needs Tesseract, which isn't installed on this computer. Pages are saved without searchable text.

# Note under the bar when some pages could not be read for text. The page numbers follow.
scan-ocr-unread =
    { $n ->
        [one] The text on page { $list } could not be read. That page is saved without searchable text.
       *[other] The text on pages { $list } could not be read. Those pages are saved without searchable text.
    }

# Note under the bar: what was saved, then what went wrong with the text.
scan-ocr-note-joined = { $saved } { $text }

# Label and spoken name of the switch that puts searchable text in saved PDFs.
scan-editor-searchable = Make text searchable

# Label above the choice of the language to read.
scan-editor-ocr-language = Language of the text

# Label above the choice of a second language to read.
scan-editor-ocr-second = Second language (optional)

# Choice of second language: none.
scan-editor-ocr-none = None

# Note under the searchable text choice.
scan-editor-ocr-note = Reading text takes a few seconds a page. If Trindle Scan can't read the text on a page, it saves the page without searchable text.

## Zoom and crop shape in the viewer (src/main.rs)

# Heading above the zoom buttons in the page viewer.
scan-view-zoom-heading = Zoom

# Buttons that make the page in the viewer larger or smaller.
scan-view-zoom-in = Zoom in
scan-view-zoom-out = Zoom out

# Button that shows the whole page, and the one that shows it at its own size.
scan-view-zoom-fit = Whole page
scan-view-zoom-actual = Actual size

# Note under the zoom buttons.
scan-view-zoom-note = You can also zoom with the mouse wheel and drag the page to move it. Plus and minus zoom in and out, and 0 shows the whole page.

# Label and spoken name of the choice of shape the crop box keeps.
scan-view-crop-shape = Crop shape

# The shapes: free, square, and width to height as 4 to 3, 3 to 2 and 16 to 9.
scan-view-crop-shape-free = Free
scan-view-crop-shape-square = Square
scan-view-crop-shape-4-3 = 4 to 3
scan-view-crop-shape-3-2 = 3 to 2
scan-view-crop-shape-16-9 = 16 to 9

## Closing the window (src/main.rs)

# Heading when the window is closed with pages not saved or a scan running.
scan-close-title = Close without saving?

# How many pages are not saved yet.
scan-close-pages =
    { $n ->
        [one] One page is not saved.
       *[other] { $n } pages are not saved.
    }

# Said when a scan is still running as the window is closed.
scan-close-scanning = A scan is still running.

# Button that closes the window and loses the unsaved pages.
scan-close-discard = Discard and close

# Button that goes back to the pages.
scan-close-keep = Keep working

# Printed in the terminal, not shown in a window, when the program is started without a desktop to draw on.
scan-main-start-no-desktop = Trindle Scan can't open a window because no desktop is running.
scan-main-start-no-window = Trindle Scan couldn't open a window on the desktop.
