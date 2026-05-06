pub const EPISODE_CELL_WIDTH: usize = 9;

pub fn calculate_grid_cols(available_width: u16) -> usize {
    let ep_area_w = (available_width as f32 * 0.6) as u16;
    let inner_w = ep_area_w.saturating_sub(4) as usize;
    (inner_w / EPISODE_CELL_WIDTH).max(1)
}
