# Spec Delta

## Purpose

The self-contained File > Automate and File > Scripts commands that batch, lay out, or package images into new documents and files.

## ADDED Requirements

### Requirement: Automate command availability

The application SHALL implement `File > Scripts > Image Processor…`, `File > Automate > Contact Sheet II…`, `File > Automate > PDF Presentation…`, `File > Automate > Picture Package…`, and `File > Automate > Web Photo Gallery…`, opening their dialogs from the documented menu paths.

`File > Automate > Conditional Mode Change…` and `File > Automate > Photomerge…` SHALL remain unavailable with an explicit reason until their prerequisite engines exist. File Naming SHALL remain unavailable because it is a Batch option (#618), not a command. None of the three SHALL apply a partial or divergent behavior.

Picture Package and Web Photo Gallery are optional 32-bit plug-ins in CS6; Kooka provides both as built-in commands. That availability difference is a recorded deviation. The Image Processor's Run Action and Camera Raw inputs are ceilings tied to #616 and the missing raw reader.

#### Scenario: The five implemented commands open their dialogs

- **WHEN** each of Image Processor, Contact Sheet II, PDF Presentation, Picture Package, and Web Photo Gallery is triggered
- **THEN** its dialog opens and no document is mutated before the dialog is accepted

#### Scenario: The deferred commands stay unavailable

- **WHEN** Conditional Mode Change or Photomerge is inspected or triggered, or the Batch File Naming option is sought
- **THEN** each is unavailable with a prerequisite reason and no document changes

### Requirement: Image Processor

Image Processor SHALL process either the open documents or a chosen folder (optionally including subfolders) and write the selected output formats. It SHALL support JPEG, PSD, and TIFF simultaneously and at least one format must be selected. Each selected format SHALL be written under a `JPEG/`, `PSD/`, or `TIFF/` subfolder of the chosen destination.

The dialog SHALL support a JPEG quality on Photoshop's 0–12 scale mapped to the encoder's 0–100 scale, resize-to-fit with the aspect ratio preserved, conversion to sRGB, and copyright metadata written through the metadata-aware save path. An unreadable input SHALL be reported and skipped without aborting the remaining files. Image Processor SHALL NOT require an action; Run Action is deferred until #616. Camera Raw input is not supported because the project has no raw container reader/demosaic.

#### Scenario: A folder produces the three formats

- **WHEN** a folder containing a readable PSD and JPEG is processed with JPEG, PSD, and TIFF selected
- **THEN** the destination contains `JPEG/`, `PSD/`, and `TIFF/`, each folder holds one output per input, and each file is a valid image in its declared format

#### Scenario: Resize to fit

- **WHEN** resize-to-fit is set to a box smaller than the source and the image is processed
- **THEN** the output fits inside the box with the source aspect ratio preserved

#### Scenario: An unreadable input is skipped

- **WHEN** one input in a folder cannot be decoded
- **THEN** the remaining inputs are still processed and the run reports the skipped file

#### Scenario: Copyright and profile are written

- **WHEN** copyright metadata and sRGB conversion are enabled
- **THEN** the written output carries the copyright field and the sRGB profile

### Requirement: Contact Sheet II

Contact Sheet II SHALL build a new document from a chosen folder or image list with document width, height, resolution, and mode; columns and rows; Place across/down; auto-spacing; rotate-for-best-fit; filename captions; and a flatten option.

The sheet SHALL place one scaled copy of each image in the chosen order inside the grid. Captions SHALL show the source filename when enabled. Flatten SHALL collapse the result to one layer; otherwise each image and caption SHALL remain its own layer.

#### Scenario: A 2×2 grid places four images

- **WHEN** four readable images are placed in a 2-column by 2-row contact sheet with Place across
- **THEN** the new document contains four image layers at the four grid cells in row-major order

#### Scenario: Place down changes the order

- **WHEN** Place down is selected for the same four images
- **THEN** the images fill the column first and the layer order follows that placement

#### Scenario: Filename captions

- **WHEN** Use Filename As Caption is enabled
- **THEN** each placed image has a caption containing its source filename

#### Scenario: Flatten collapses the sheet

- **WHEN** Flatten All Layers is enabled
- **THEN** the result contains one layer and the composite is unchanged from the unflattened sheet

### Requirement: PDF Presentation

PDF Presentation SHALL accept an ordered list of images with add, reorder, and duplicate operations, and SHALL write a multi-page PDF in list order. Each listed image SHALL be one page, including duplicates. The existing single-document `Save As` PDF path SHALL keep writing one page. Slide-show output and the full Output/Presentation option set are recorded ceilings.

#### Scenario: Order and duplicates are preserved

- **WHEN** a list of three images is reordered and one entry is duplicated, then written
- **THEN** the PDF has four pages in the displayed list order, including the duplicate

#### Scenario: The page count matches the list

- **WHEN** a presentation with N images is written
- **THEN** the PDF reports exactly N pages, with page sizes derived from each image's captured resolution

#### Scenario: Save As stays one page

- **WHEN** a single document is saved through the normal Save As PDF path
- **THEN** the PDF has one page and the existing PDF tests are unchanged

### Requirement: Picture Package

Picture Package SHALL build a page from one or more source images using the built-in layouts, with page size, resolution, mode, source File/Folder selection, and a flatten option. The built-in layouts SHALL place the requested number of copies; labels, custom layout files, and the optional plug-in install flow are outside this change.

#### Scenario: A built-in layout places the copies

- **WHEN** a single source is placed with a built-in layout that holds four copies
- **THEN** the new document contains four distinct placed copies inside the page bounds

#### Scenario: Page geometry follows the options

- **WHEN** a page size, resolution, and mode are chosen
- **THEN** the resulting document matches those values

#### Scenario: Flatten collapses the package

- **WHEN** Flatten is enabled
- **THEN** the result contains one layer and the composite matches the unflattened package

### Requirement: Web Photo Gallery

Web Photo Gallery SHALL generate a browsable gallery from a folder or image list into a chosen destination using a built-in style. The destination SHALL contain `index.htm` or `index.html`, an `images/` directory, a `pages/` directory, and a `thumbnails/` directory, with relative links that resolve. Adobe's gallery templates SHALL NOT be copied. The full CS6 General/Banner/Large Images/Thumbnails/Colors/Security option set and multiple styles are recorded ceilings.

#### Scenario: The output tree is complete

- **WHEN** a gallery is generated for two images
- **THEN** the destination contains the index file and the three directories with the generated image, page, and thumbnail files

#### Scenario: Links resolve

- **WHEN** the generated index and page files are inspected
- **THEN** every local image and page link resolves to a file in the output tree
