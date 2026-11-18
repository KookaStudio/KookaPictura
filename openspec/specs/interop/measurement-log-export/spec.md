# Measurement Log Export Specification

## Purpose

Serialize recorded measurement rows and optional histograms into portable files for use outside the application.

## Requirements

### Requirement: Tab-delimited measurement rows

The measurement export writer SHALL accept the selected rows and ordered output
columns from its caller and emit UTF-8 tab-delimited text with a header row. It
SHALL retain caller row order, caller column order, and each row’s recorded
values and scale. Values containing a tab, newline, carriage return, or double
quote SHALL be quoted with internal double quotes doubled. Numeric output SHALL
use locale-independent decimal formatting. An empty row selection SHALL leave
the filesystem untouched.

#### Scenario: Selected measurements retain their recorded scales [measurement_tsv]

- **WHEN** a caller exports an ordered subset of rows with different recorded scales, reordered columns, and Unicode or delimiter-bearing values
- **THEN** the UTF-8 file contains the ordered headers and exactly those rows with their original values, correctly quoted

#### Scenario: Empty selections produce no export [measurement_export_empty]

- **WHEN** no rows are selected
- **THEN** no table or histogram destination is created or modified

### Requirement: Histogram CSV artifacts

When the selected columns and rows include histograms, the writer SHALL produce
one CSV per exported histogram in a sibling `<export-stem>-histograms` directory.
Files SHALL be named `0.csv`, `1.csv`, and so on, in exported row order. The
table’s Histogram cells SHALL carry relative paths for those files and empty
values for rows without histograms. Each file SHALL have one row per bin from
0 to 255 and one count column per recorded
channel, preserving channel order. A selection with no exported histograms
SHALL produce no histogram directory.

#### Scenario: Histograms follow the exported row order [measurement_histograms]

- **WHEN** the exported rows contain a summary histogram, rows without histograms, and individual feature histograms
- **THEN** the sibling directory contains consecutive CSVs for the histogram-bearing rows, with their channel counts in the original order

#### Scenario: The histogram data point is omitted [measurement_without_histograms]

- **WHEN** the selected rows have histogram data and the caller omits the Histogram column
- **THEN** the table is written without any histogram directory

### Requirement: Export destination safety

The writer SHALL report a conflicting destination or write failure and SHALL
preserve pre-existing files and folders. A failed export SHALL clean up its
incomplete output where the filesystem allows and report any output it could
not remove.

#### Scenario: A conflicting or failing export leaves existing data intact [measurement_export_failure]

- **WHEN** the destination file or required histogram directory already exists, or a write fails after export begins
- **THEN** the writer reports an error, keeps existing data intact, and removes or identifies output left by the failed export
