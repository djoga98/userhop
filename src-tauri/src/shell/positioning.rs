#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelEdge {
    Bottom,
    Top,
    Left,
    Right,
}

/// Infers which edge the taskbar or panel occupies from the monitor/work-area difference.
pub fn panel_edge(monitor: Rect, work: Rect) -> PanelEdge {
    let top = i64::from(work.y) - i64::from(monitor.y);
    let left = i64::from(work.x) - i64::from(monitor.x);
    let bottom = monitor.bottom() - work.bottom();
    let right = monitor.right() - work.right();
    let max = top.max(left).max(bottom).max(right);
    if max <= 0 || bottom == max {
        PanelEdge::Bottom
    } else if top == max {
        PanelEdge::Top
    } else if right == max {
        PanelEdge::Right
    } else {
        PanelEdge::Left
    }
}

fn clamp_axis(value: i64, start: i64, end: i64, size: i64) -> i32 {
    let max = (end - size).max(start);
    let clamped = value.clamp(start, max);
    i32::try_from(clamped).unwrap_or(if clamped < 0 { i32::MIN } else { i32::MAX })
}

/// Physical top-left position that anchors the flyout next to the panel edge, right-aligned.
pub fn flyout_origin(monitor: Rect, work: Rect, size: (u32, u32), margin: u32) -> (i32, i32) {
    let (width, height) = (i64::from(size.0), i64::from(size.1));
    let margin = i64::from(margin);
    let (left, top, right, bottom) = (
        i64::from(work.x),
        i64::from(work.y),
        work.right(),
        work.bottom(),
    );
    let (x, y) = match panel_edge(monitor, work) {
        PanelEdge::Bottom => (right - width - margin, bottom - height - margin),
        PanelEdge::Top => (right - width - margin, top + margin),
        PanelEdge::Right => (right - width - margin, bottom - height - margin),
        PanelEdge::Left => (left + margin, bottom - height - margin),
    };
    (
        clamp_axis(x, left, right, width),
        clamp_axis(y, top, bottom, height),
    )
}

pub fn centered_origin(work: Rect, size: (u32, u32)) -> (i32, i32) {
    let x = i64::from(work.x) + (i64::from(work.width) - i64::from(size.0)) / 2;
    let y = i64::from(work.y) + (i64::from(work.height) - i64::from(size.1)) / 2;
    (
        clamp_axis(x, i64::from(work.x), work.right(), i64::from(size.0)),
        clamp_axis(y, i64::from(work.y), work.bottom(), i64::from(size.1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn bottom_taskbar_anchors_bottom_right() {
        let monitor = rect(0, 0, 1920, 1080);
        let work = rect(0, 0, 1920, 1032);
        assert_eq!(panel_edge(monitor, work), PanelEdge::Bottom);
        assert_eq!(flyout_origin(monitor, work, (388, 300), 12), (1520, 720));
    }

    #[test]
    fn top_left_right_taskbars() {
        let monitor = rect(0, 0, 1920, 1080);
        let top = rect(0, 48, 1920, 1032);
        assert_eq!(flyout_origin(monitor, top, (388, 300), 12), (1520, 60));
        let left = rect(60, 0, 1860, 1080);
        assert_eq!(panel_edge(monitor, left), PanelEdge::Left);
        assert_eq!(flyout_origin(monitor, left, (388, 300), 12), (72, 768));
        let right = rect(0, 0, 1860, 1080);
        assert_eq!(panel_edge(monitor, right), PanelEdge::Right);
        assert_eq!(flyout_origin(monitor, right, (388, 300), 12), (1460, 768));
    }

    #[test]
    fn negative_coordinates_and_high_dpi_monitor() {
        let monitor = rect(-3840, -200, 3840, 2160);
        let work = rect(-3840, -200, 3840, 2064);
        let size = (776, 600);
        assert_eq!(flyout_origin(monitor, work, size, 24), (-800, 1240));
    }

    #[test]
    fn hidden_or_missing_taskbar_defaults_to_bottom() {
        let monitor = rect(0, 0, 1280, 720);
        assert_eq!(panel_edge(monitor, monitor), PanelEdge::Bottom);
        assert_eq!(flyout_origin(monitor, monitor, (388, 300), 12), (880, 408));
    }

    #[test]
    fn oversized_window_is_clamped_into_work_area() {
        let monitor = rect(0, 0, 800, 600);
        let work = rect(0, 0, 800, 560);
        assert_eq!(flyout_origin(monitor, work, (900, 700), 12), (0, 0));
    }

    #[test]
    fn centered_fallback() {
        assert_eq!(
            centered_origin(rect(100, 50, 1000, 800), (400, 300)),
            (400, 300)
        );
        assert_eq!(
            centered_origin(rect(-1000, 0, 1000, 800), (400, 300)),
            (-700, 250)
        );
    }
}
