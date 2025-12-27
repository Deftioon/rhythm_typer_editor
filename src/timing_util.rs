use crate::data::{Meta, TimingPoint};

pub fn meta_timing_points_sorted(meta: &Meta) -> Vec<TimingPoint> {
    let mut points = meta.timingPoints.clone();

    // If timingPoints is empty, fall back
    if points.is_empty() {
        let id = meta.offset as f64;
        let bpm = if meta.bpm > 0.0 { meta.bpm } else { 120.0 };
        points.push(TimingPoint {
            id,
            time: (meta.offset as f64) / 1000.0,
            bpm,
            offset: meta.offset,
            timeSignature: [4, 4],
        });
    }

    points.sort_by(|a, b| a.offset.cmp(&b.offset));
    points
}

pub fn timing_point_at(points: &[TimingPoint], time_ms: u32) -> TimingPoint {
    // Return the most recent timing point with offset <= time_ms
    let t = time_ms as i64;
    let mut active: Option<&TimingPoint> = None;
    for p in points {
        if p.offset <= t {
            active = Some(p);
        } else {
            break;
        }
    }
    active.cloned().unwrap_or_else(|| points[0].clone())
}

pub fn beat_len_ms(tp: &TimingPoint) -> f32 {
    let bpm: f32 = (tp.bpm as f32).max(1.0);
    // Interpret BPM as quarter-notes per minute and derive beat length from the
    // time signature denominator so timing points can change the grid feel.
    let quarter_len: f32 = 60_000.0 / bpm;
    let denom = tp.timeSignature[1].max(1) as f32;
    (quarter_len * (4.0 / denom)).max(1.0)
}

// beat_divisor of 2 means half-beat (the 1/8 grid in 4/4).
pub fn snap_time_to_beat_divisor_ms(points: &[TimingPoint], time_ms: u32, beat_divisor: u32) -> u32 {
    if points.is_empty() {
        return time_ms;
    }

    let divisor = beat_divisor.max(1) as f32;
    let tp = timing_point_at(points, time_ms);
    let seg_start = tp.offset as f32;

    let beat_len = beat_len_ms(&tp);
    let step = (beat_len / divisor).max(1.0);

    let t = time_ms as f32;
    let k = ((t - seg_start) / step).round();
    let snapped = seg_start + k * step;
    snapped.max(0.0).round() as u32
}
