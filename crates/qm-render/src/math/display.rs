//! Display math: stacked rows whose cells align in columns like TeX's `align`.

use qm_format::DisplayMath;

use super::list::ListOptions;
use super::{MathBox, MathLayout, MathStyle};

/// Space between column pairs of an aligned display (`x &= 1 & y &= 2`).
const COLUMN_PAIR_GAP: f32 = 2.0;
/// TeX's baselineskip plus `\jot` between display rows.
const ROW_MIN_PITCH: f32 = 1.5;
const ROW_MIN_GAP: f32 = 0.3;
/// Empty rows keep a strut so the block doesn't collapse.
const ROW_STRUT_ASCENT: f32 = 0.7;
const ROW_STRUT_DESCENT: f32 = 0.25;

/// Lays out `math` with the first row's baseline at y = 0.
///
/// Rows with a single cell are centred individually; when any row has
/// several cells, even-numbered cells (0, 2, …) are right-aligned and odd ones
/// left-aligned so the alignment points line up.
pub fn layout_display(layout: &mut MathLayout<'_>, math: &DisplayMath) -> MathBox {
    let size = layout.size(MathStyle::DISPLAY);
    let rows: Vec<Vec<MathBox>> = math
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .enumerate()
                .map(|(column, cell)| {
                    let options = ListOptions {
                        starts_after_ord: column % 2 == 1,
                        ..Default::default()
                    };
                    layout.list_with(cell, MathStyle::DISPLAY, options)
                })
                .collect()
        })
        .collect();
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let column_x = column_positions(&rows, column_count, size);
    let total_width = column_x.last().copied().unwrap_or(0.0);

    let mut result = MathBox {
        width: total_width,
        ..MathBox::default()
    };
    let mut baseline: Option<f32> = None;
    let mut previous_descent = 0.0;
    for row in &rows {
        let ascent = row
            .iter()
            .map(|c| c.ascent)
            .fold(ROW_STRUT_ASCENT * size, f32::max);
        let descent = row
            .iter()
            .map(|c| c.descent)
            .fold(ROW_STRUT_DESCENT * size, f32::max);
        let y = match baseline {
            None => 0.0,
            Some(y) => {
                y + (previous_descent + ROW_MIN_GAP * size + ascent).max(ROW_MIN_PITCH * size)
            }
        };
        place_row(&mut result, row, &column_x, column_count, y);
        result.ascent = result.ascent.max(ascent - y);
        result.descent = result.descent.max(descent + y);
        baseline = Some(y);
        previous_descent = descent;
    }
    result
}

/// Left edges of each column plus the total width as the last entry.
fn column_positions(rows: &[Vec<MathBox>], column_count: usize, size: f32) -> Vec<f32> {
    if column_count <= 1 {
        let width = rows.iter().flatten().map(|c| c.width).fold(0.0, f32::max);
        return vec![0.0, width];
    }
    let mut widths = vec![0.0f32; column_count];
    for row in rows {
        for (column, cell) in row.iter().enumerate() {
            widths[column] = widths[column].max(cell.width);
        }
    }
    let mut positions = Vec::with_capacity(column_count + 1);
    let mut x = 0.0;
    for (column, width) in widths.iter().enumerate() {
        if column > 0 && column % 2 == 0 {
            x += COLUMN_PAIR_GAP * size;
        }
        positions.push(x);
        x += width;
    }
    positions.push(x);
    positions
}

fn place_row(result: &mut MathBox, row: &[MathBox], column_x: &[f32], column_count: usize, y: f32) {
    if column_count <= 1 {
        if let Some(cell) = row.first() {
            result.place(cell, (result.width - cell.width) / 2.0, y);
        }
        return;
    }
    for (column, cell) in row.iter().enumerate() {
        let x = if column % 2 == 0 {
            column_x[column + 1] - cell.width
        } else {
            column_x[column]
        };
        result.place(cell, x, y);
    }
}
