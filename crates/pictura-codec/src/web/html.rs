//! Save for Web slice HTML/CSS: the table or absolutely positioned layout that
//! reassembles the exported slice images at their canvas positions.

/// One exported slice image, placed in canvas pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SliceCell {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceLayout {
    Table,
    Css,
}

pub struct SliceHtmlOptions<'a> {
    pub title: &'a str,
    pub image_dir: &'a str,
    pub layout: SliceLayout,
}

/// Emit the Save for Web slice HTML for `cells` on a `canvas_width` x
/// `canvas_height` document. Zero-area cells are omitted.
pub fn slice_html(
    canvas_width: u32,
    canvas_height: u32,
    cells: &[SliceCell],
    options: &SliceHtmlOptions,
) -> String {
    let cells: Vec<&SliceCell> = cells
        .iter()
        .filter(|c| c.width > 0 && c.height > 0 && c.x < canvas_width && c.y < canvas_height)
        .collect();
    let title = escape_text(options.title);
    let mut out = String::new();
    out.push_str("<html>\n<head>\n<title>");
    out.push_str(&title);
    out.push_str(
        "</title>\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">\n",
    );
    match options.layout {
        SliceLayout::Table => {}
        SliceLayout::Css => push_css_style(&mut out, canvas_width, canvas_height, &cells),
    }
    out.push_str("</head>\n");
    out.push_str(
        "<body bgcolor=\"#FFFFFF\" leftmargin=\"0\" topmargin=\"0\" marginwidth=\"0\" marginheight=\"0\">\n",
    );
    out.push_str("<!-- Save for Web Slices (");
    out.push_str(&title);
    out.push_str(") -->\n");
    match options.layout {
        SliceLayout::Table => push_table(
            &mut out,
            canvas_width,
            canvas_height,
            &cells,
            options.image_dir,
        ),
        SliceLayout::Css => push_css_body(&mut out, &cells, options.image_dir),
    }
    out.push_str("<!-- End Save for Web Slices -->\n</body>\n</html>\n");
    out
}

fn push_table(
    out: &mut String,
    canvas_width: u32,
    canvas_height: u32,
    cells: &[&SliceCell],
    image_dir: &str,
) {
    let (xs, ys) = grid(canvas_width, canvas_height, cells);
    out.push_str(&format!(
        "<table id=\"Table_01\" width=\"{canvas_width}\" height=\"{canvas_height}\" border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\n"
    ));
    for band in ys.windows(2) {
        let top = band[0];
        out.push_str("  <tr>\n");
        let mut xi = 0;
        while xi + 1 < xs.len() {
            let left = xs[xi];
            let Some(cell) = cells.iter().find(|c| c.x == left && c.y == top) else {
                xi += 1;
                continue;
            };
            let right = cell.x.saturating_add(cell.width).min(canvas_width);
            let bottom = cell.y.saturating_add(cell.height).min(canvas_height);
            let end_x = xs.iter().position(|&x| x == right).unwrap_or(xi + 1);
            let end_y = ys.iter().position(|&y| y == bottom).unwrap_or(ys.len() - 1);
            let colspan = end_x.saturating_sub(xi).max(1);
            let rowspan = end_y
                .saturating_sub(ys.iter().position(|&y| y == top).unwrap_or(0))
                .max(1);
            out.push_str("    <td");
            if colspan > 1 {
                out.push_str(&format!(" colspan=\"{colspan}\""));
            }
            if rowspan > 1 {
                out.push_str(&format!(" rowspan=\"{rowspan}\""));
            }
            out.push_str(">\n      <img src=\"");
            out.push_str(&image_url(image_dir, &cell.name));
            out.push_str(&format!(
                "\" width=\"{}\" height=\"{}\" alt=\"\" style=\"display:block\"></td>\n",
                cell.width, cell.height
            ));
            xi = end_x.max(xi + 1);
        }
        out.push_str("  </tr>\n");
    }
    out.push_str("</table>\n");
}

fn push_css_style(out: &mut String, canvas_width: u32, canvas_height: u32, cells: &[&SliceCell]) {
    out.push_str("<style type=\"text/css\">\n<!--\n");
    out.push_str(&format!(
        "#Table_01 {{ position:relative; left:0px; top:0px; width:{canvas_width}px; height:{canvas_height}px; }}\n"
    ));
    for cell in cells {
        out.push_str(&format!(
            "#{} {{ position:absolute; left:{}px; top:{}px; width:{}px; height:{}px; }}\n",
            css_id(&cell.name),
            cell.x,
            cell.y,
            cell.width,
            cell.height
        ));
    }
    out.push_str("-->\n</style>\n");
}

fn push_css_body(out: &mut String, cells: &[&SliceCell], image_dir: &str) {
    out.push_str("<div id=\"Table_01\">\n");
    for cell in cells {
        out.push_str(&format!("  <div id=\"{}\">\n", css_id(&cell.name)));
        out.push_str(&format!(
            "    <img src=\"{}\" width=\"{}\" height=\"{}\" alt=\"\" style=\"display:block\"></div>\n",
            image_url(image_dir, &cell.name),
            cell.width,
            cell.height
        ));
    }
    out.push_str("</div>\n");
}

fn grid(canvas_width: u32, canvas_height: u32, cells: &[&SliceCell]) -> (Vec<u32>, Vec<u32>) {
    let mut xs = vec![0, canvas_width];
    let mut ys = vec![0, canvas_height];
    for cell in cells {
        xs.push(cell.x.min(canvas_width));
        xs.push(cell.x.saturating_add(cell.width).min(canvas_width));
        ys.push(cell.y.min(canvas_height));
        ys.push(cell.y.saturating_add(cell.height).min(canvas_height));
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    (xs, ys)
}

fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn image_url(image_dir: &str, name: &str) -> String {
    let path = if image_dir.is_empty() {
        name.to_string()
    } else {
        format!("{image_dir}/{name}")
    };
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn css_id(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let mut id: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if id.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        id.insert_str(0, "s_");
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(name: &str, x: u32, y: u32, width: u32, height: u32) -> SliceCell {
        SliceCell {
            name: name.to_string(),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn html_table_grid() {
        let cells = [
            cell("a.gif", 0, 0, 2, 2),
            cell("b.gif", 2, 0, 2, 2),
            cell("c.gif", 0, 2, 2, 2),
            cell("d.gif", 2, 2, 2, 2),
        ];
        let html = slice_html(
            4,
            4,
            &cells,
            &SliceHtmlOptions {
                title: "Doc",
                image_dir: "images",
                layout: SliceLayout::Table,
            },
        );
        assert!(html.contains("<table id=\"Table_01\" width=\"4\" height=\"4\""));
        assert!(html.contains("<title>Doc</title>"));
        assert_eq!(html.matches("<img src=\"images/").count(), 4);
        assert_eq!(html.matches("style=\"display:block\"").count(), 4);
        assert!(html.contains("src=\"images/a.gif\" width=\"2\" height=\"2\""));
        assert!(html.contains("<!-- Save for Web Slices (Doc) -->"));
        assert!(html.contains("<!-- End Save for Web Slices -->"));
    }

    #[test]
    fn html_table_spans_a_tall_slice() {
        let cells = [
            cell("tall.gif", 0, 0, 2, 4),
            cell("top.gif", 2, 0, 2, 2),
            cell("bottom.gif", 2, 2, 2, 2),
        ];
        let html = slice_html(
            4,
            4,
            &cells,
            &SliceHtmlOptions {
                title: "Doc",
                image_dir: "images",
                layout: SliceLayout::Table,
            },
        );
        assert!(html.contains("rowspan=\"2\""));
        assert!(html.contains("tall.gif"));
    }

    #[test]
    fn html_css_grid() {
        let cells = [cell("a.gif", 0, 0, 2, 2), cell("b.gif", 2, 0, 2, 2)];
        let html = slice_html(
            4,
            2,
            &cells,
            &SliceHtmlOptions {
                title: "Doc",
                image_dir: "images",
                layout: SliceLayout::Css,
            },
        );
        assert!(!html.contains("<table"));
        assert!(html.contains(
            "#Table_01 { position:relative; left:0px; top:0px; width:4px; height:2px; }"
        ));
        assert!(
            html.contains("#a { position:absolute; left:0px; top:0px; width:2px; height:2px; }")
        );
        assert!(
            html.contains("#b { position:absolute; left:2px; top:0px; width:2px; height:2px; }")
        );
        assert!(html.contains("<div id=\"Table_01\">"));
        assert_eq!(html.matches("<img src=\"images/").count(), 2);
        assert_eq!(html.matches("style=\"display:block\"").count(), 2);
    }

    #[test]
    fn css_id_prefixes_a_leading_digit() {
        let cells = [cell("3d.gif", 0, 0, 2, 2)];
        let html = slice_html(
            2,
            2,
            &cells,
            &SliceHtmlOptions {
                title: "Doc",
                image_dir: "images",
                layout: SliceLayout::Css,
            },
        );
        assert!(html.contains("#s_3d {"));
        assert!(!html.contains("#3d {"));
    }

    #[test]
    fn html_escaping() {
        let cells = [cell("slice 1&.gif", 0, 0, 2, 2)];
        let html = slice_html(
            2,
            2,
            &cells,
            &SliceHtmlOptions {
                title: "a&\"<>",
                image_dir: "im ages",
                layout: SliceLayout::Css,
            },
        );
        assert!(html.contains("a&amp;&quot;&lt;&gt;"));
        assert!(html.contains("im%20ages/slice%201%26.gif"));
        assert!(html.contains("#slice_1_"));
    }

    #[test]
    fn zero_area_cells_are_omitted() {
        let cells = [cell("a.gif", 0, 0, 2, 2), cell("zero.gif", 2, 0, 0, 2)];
        let html = slice_html(
            2,
            2,
            &cells,
            &SliceHtmlOptions {
                title: "Doc",
                image_dir: "images",
                layout: SliceLayout::Table,
            },
        );
        assert!(html.contains("a.gif"));
        assert!(!html.contains("zero.gif"));
    }
}
