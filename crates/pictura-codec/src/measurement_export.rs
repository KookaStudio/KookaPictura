//! Export the caller's projection of already recorded measurement rows.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

pub struct HistogramChannel<'a> {
    pub name: &'a str,
    pub bins: &'a [u64; 256],
}

pub enum MeasurementCell<'a> {
    Empty,
    Text(&'a str),
    Integer(u64),
    Number(f64),
    Histogram(&'a [HistogramChannel<'a>]),
}

#[derive(Debug)]
pub struct MeasurementExportError {
    error: io::Error,
    incomplete: Vec<(PathBuf, io::Error)>,
}

impl fmt::Display for MeasurementExportError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}", self.error)?;
        for (path, error) in &self.incomplete {
            write!(output, "\nCould not remove {} ({error})", path.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for MeasurementExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Create a UTF-8 TSV and any required sibling histogram files. Each row
/// supplies one cell per output header. The caller projects the requested rows
/// and columns before export. Existing destinations are preserved.
pub fn export_measurements(
    destination: &Path,
    headers: &[&str],
    rows: &[&[MeasurementCell<'_>]],
) -> Result<(), MeasurementExportError> {
    export_with(destination, headers, rows, write_table)
}

#[derive(Default)]
struct ExportFiles {
    table: bool,
    histogram_dir: Option<PathBuf>,
    histograms: Vec<PathBuf>,
}

impl ExportFiles {
    fn remove(self, table: &Path) -> Vec<(PathBuf, io::Error)> {
        let mut incomplete = Vec::new();
        let mut record_error = |path: &Path, result: io::Result<()>| {
            if let Err(error) = result {
                if error.kind() != io::ErrorKind::NotFound {
                    incomplete.push((path.to_owned(), error));
                }
            }
        };
        if self.table {
            record_error(table, fs::remove_file(table));
        }
        for path in self.histograms {
            record_error(&path, fs::remove_file(&path));
        }
        if let Some(dir) = self.histogram_dir {
            record_error(&dir, fs::remove_dir(&dir));
        }
        incomplete
    }
}

fn export_with(
    destination: &Path,
    headers: &[&str],
    rows: &[&[MeasurementCell<'_>]],
    table_writer: impl FnOnce(
        &mut dyn Write,
        &[&str],
        &[&[MeasurementCell<'_>]],
        &str,
    ) -> io::Result<()>,
) -> Result<(), MeasurementExportError> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut created = ExportFiles::default();
    let result = (|| {
        if rows.iter().any(|row| row.len() != headers.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Measurement row does not match the export columns",
            ));
        }
        let histograms = rows
            .iter()
            .flat_map(|row| row.iter())
            .filter_map(|cell| match cell {
                MeasurementCell::Histogram(channels) => Some(*channels),
                _ => None,
            });
        let histogram_name = if histograms.clone().next().is_some() {
            let stem = destination
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Histogram export needs a UTF-8 filename",
                    )
                })?;
            format!("{stem}-histograms")
        } else {
            String::new()
        };

        let mut table = BufWriter::new(File::create_new(destination)?);
        created.table = true;
        if !histogram_name.is_empty() {
            let directory = destination.with_file_name(&histogram_name);
            fs::create_dir(&directory)?;
            created.histogram_dir = Some(directory.clone());
            for (index, channels) in histograms.enumerate() {
                let path = directory.join(format!("{index}.csv"));
                let mut file = BufWriter::new(File::create_new(&path)?);
                created.histograms.push(path);
                write_histogram(&mut file, channels)?;
                file.flush()?;
            }
        }
        table_writer(&mut table, headers, rows, &histogram_name)?;
        table.flush()
    })();
    result.map_err(|error| MeasurementExportError {
        error,
        incomplete: created.remove(destination),
    })
}

fn write_field(out: &mut dyn Write, text: &str, delimiter: u8) -> io::Result<()> {
    if !text
        .bytes()
        .any(|byte| matches!(byte, b'"' | b'\r' | b'\n') || byte == delimiter)
    {
        return out.write_all(text.as_bytes());
    }
    out.write_all(b"\"")?;
    for (index, part) in text.split('"').enumerate() {
        if index > 0 {
            out.write_all(b"\"\"")?;
        }
        out.write_all(part.as_bytes())?;
    }
    out.write_all(b"\"")
}

fn write_table(
    out: &mut dyn Write,
    headers: &[&str],
    rows: &[&[MeasurementCell<'_>]],
    histogram_name: &str,
) -> io::Result<()> {
    for (index, header) in headers.iter().enumerate() {
        if index > 0 {
            out.write_all(b"\t")?;
        }
        write_field(out, header, b'\t')?;
    }
    out.write_all(b"\n")?;
    let mut histogram_index = 0;
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index > 0 {
                out.write_all(b"\t")?;
            }
            match cell {
                MeasurementCell::Empty => {}
                MeasurementCell::Text(text) => write_field(out, text, b'\t')?,
                MeasurementCell::Integer(value) => write!(out, "{value}")?,
                MeasurementCell::Number(value) => write!(out, "{value}")?,
                MeasurementCell::Histogram(_) => {
                    let filename = format!("{histogram_name}/{histogram_index}.csv");
                    write_field(out, &filename, b'\t')?;
                    histogram_index += 1;
                }
            }
        }
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn write_histogram(out: &mut dyn Write, channels: &[HistogramChannel<'_>]) -> io::Result<()> {
    out.write_all(b"Bin")?;
    for channel in channels {
        out.write_all(b",")?;
        write_field(out, channel.name, b',')?;
    }
    out.write_all(b"\n")?;
    for bin in 0..256 {
        write!(out, "{bin}")?;
        for channel in channels {
            write!(out, ",{}", channel.bins[bin])?;
        }
        out.write_all(b"\n")?;
    }
    Ok(())
}
