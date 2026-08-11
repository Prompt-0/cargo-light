#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone)]
pub struct TreemapItem {
    pub name: String,
    pub value: f64,
    pub formatted_value: String,
    pub rect: Rect,
    pub color_idx: u8,
}

/// Computes squarified / slice-and-dice treemap layout for items within container `rect`
pub fn compute_treemap_layout(
    items: &[(String, f64, String)],
    rect: Rect,
) -> Vec<TreemapItem> {
    if items.is_empty() || rect.width == 0 || rect.height == 0 {
        return Vec::new();
    }

    let total_val: f64 = items.iter().map(|(_, v, _)| *v).sum();
    if total_val <= 0.0 {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(items.len());
    let mut curr_rect = rect;

    let mut color_counter: u8 = 0;

    for (name, val, formatted) in items {
        if curr_rect.width == 0 || curr_rect.height == 0 {
            break;
        }

        let ratio = (*val / total_val).clamp(0.01, 1.0);

        let (item_rect, remaining_rect) = if curr_rect.width >= curr_rect.height {
            // Split vertically
            let item_w = ((curr_rect.width as f64) * ratio).round() as u16;
            let item_w = item_w.clamp(1, curr_rect.width);
            let item = Rect {
                x: curr_rect.x,
                y: curr_rect.y,
                width: item_w,
                height: curr_rect.height,
            };
            let rem = Rect {
                x: curr_rect.x + item_w,
                y: curr_rect.y,
                width: curr_rect.width.saturating_sub(item_w),
                height: curr_rect.height,
            };
            (item, rem)
        } else {
            // Split horizontally
            let item_h = ((curr_rect.height as f64) * ratio).round() as u16;
            let item_h = item_h.clamp(1, curr_rect.height);
            let item = Rect {
                x: curr_rect.x,
                y: curr_rect.y,
                width: curr_rect.width,
                height: item_h,
            };
            let rem = Rect {
                x: curr_rect.x,
                y: curr_rect.y + item_h,
                width: curr_rect.width,
                height: curr_rect.height.saturating_sub(item_h),
            };
            (item, rem)
        };

        result.push(TreemapItem {
            name: name.clone(),
            value: *val,
            formatted_value: formatted.clone(),
            rect: item_rect,
            color_idx: color_counter,
        });

        color_counter = (color_counter + 1) % 6;
        curr_rect = remaining_rect;
    }

    result
}

/// Generates an ASCII treemap representation for terminal text mode
pub fn render_ascii_treemap(items: &[(String, f64, String)], width: usize) -> String {
    let mut out = String::new();
    let total_val: f64 = items.iter().map(|(_, v, _)| *v).sum();
    if total_val <= 0.0 {
        return out;
    }

    out.push_str("┌─ Dependency Size & Weight Footprint Treemap ──────────────────────────────────┐\n");

    let bar_width = width.saturating_sub(25).max(20);

    for (name, val, formatted) in items {
        let pct = (val / total_val) * 100.0;
        let filled_len = (((val / total_val) * (bar_width as f64)).round() as usize).clamp(1, bar_width);
        let empty_len = bar_width.saturating_sub(filled_len);

        let bar = format!(
            "[{}{}]",
            "█".repeat(filled_len),
            "░".repeat(empty_len)
        );

        out.push_str(&format!(
            "│ {:<18} {:<width$} {:>10} ({:4.1}%) │\n",
            name,
            bar,
            formatted,
            pct,
            width = bar_width + 2
        ));
    }

    out.push_str("└───────────────────────────────────────────────────────────────────────────────┘\n");
    out
}
