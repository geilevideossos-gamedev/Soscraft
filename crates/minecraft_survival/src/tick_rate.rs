use std::time::{Duration, Instant};

use minecraft_commands::{SprintReport, TickQuery};

pub const SAMPLES: usize = 100;
pub const DEFAULT_RATE: f32 = 20.0;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const NANOS_PER_MILLISECOND: u64 = 1_000_000;
const OVERLOADED_THRESHOLD_NANOS: u64 = 20 * NANOS_PER_SECOND / 20;
const OVERLOADED_WARNING_INTERVAL_NANOS: u64 = 10 * NANOS_PER_SECOND;
const BATCH_BUDGET_NANOS: u64 = 50_000_000;
const MAX_BATCH: u64 = 100;
const CLIENT_TICK_SECONDS: f64 = 0.05;
pub const MIN_RATE: f32 = 1.0;
pub const AUTOSAVE_INTERVAL: i32 = 6000;
const MIN_AUTOSAVE_TICKS: i32 = 100;
const AUTOSAVE_SECONDS: f32 = 300.0;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TickBatch {
    pub normal: Vec<bool>,
    pub overload: Option<(u64, u64)>,
}

impl TickBatch {
    pub fn len(&self) -> usize {
        self.normal.len()
    }

    pub fn is_empty(&self) -> bool {
        self.normal.is_empty()
    }

    pub fn normal_count(&self) -> u32 {
        self.normal.iter().filter(|&&normal| normal).count() as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientTiming {
    pub tick_seconds: f64,
    pub runs_normally: bool,
}

impl Default for ClientTiming {
    fn default() -> Self {
        Self {
            tick_seconds: CLIENT_TICK_SECONDS,
            runs_normally: true,
        }
    }
}

pub struct TickRate {
    rate: f32,
    nanos_per_tick: u64,
    frozen: bool,
    frozen_ticks_to_run: i32,
    run_game_elements: bool,
    remaining_sprint_ticks: i64,
    scheduled_sprint_ticks: i64,
    sprint_time_spent_nanos: u64,
    previous_frozen: bool,
    next_tick: Option<Instant>,
    last_overload_warning: Option<Instant>,
    tick_count: u64,
    recorded: u64,
    samples: [u64; SAMPLES],
    aggregated: u64,
    last_batch_nanos: u64,
    ticks_until_autosave: i32,
    reports: Vec<SprintReport>,
}

impl Default for TickRate {
    fn default() -> Self {
        Self {
            rate: DEFAULT_RATE,
            nanos_per_tick: (NANOS_PER_SECOND as f64 / f64::from(DEFAULT_RATE)) as u64,
            frozen: false,
            frozen_ticks_to_run: 0,
            run_game_elements: true,
            remaining_sprint_ticks: 0,
            scheduled_sprint_ticks: 0,
            sprint_time_spent_nanos: 0,
            previous_frozen: false,
            next_tick: None,
            last_overload_warning: None,
            tick_count: 0,
            recorded: 0,
            samples: [0; SAMPLES],
            aggregated: 0,
            last_batch_nanos: 1,
            ticks_until_autosave: AUTOSAVE_INTERVAL,
            reports: Vec::new(),
        }
    }
}

impl TickRate {
    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn nanos_per_tick(&self) -> u64 {
        self.nanos_per_tick
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn runs_normally(&self) -> bool {
        self.run_game_elements
    }

    pub fn is_sprinting(&self) -> bool {
        self.scheduled_sprint_ticks > 0
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.max(MIN_RATE);
        self.nanos_per_tick = (NANOS_PER_SECOND as f64 / f64::from(self.rate)) as u64;
        self.on_rate_changed();
    }

    pub fn client_timing(&self) -> ClientTiming {
        if !self.run_game_elements {
            return ClientTiming {
                tick_seconds: CLIENT_TICK_SECONDS,
                runs_normally: false,
            };
        }
        ClientTiming {
            tick_seconds: (self.nanos_per_tick as f64 / NANOS_PER_SECOND as f64)
                .max(CLIENT_TICK_SECONDS),
            runs_normally: true,
        }
    }

    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn is_stepping(&self) -> bool {
        self.frozen_ticks_to_run > 0
    }

    pub fn step(&mut self, ticks: i32) -> bool {
        if !self.frozen {
            return false;
        }
        self.frozen_ticks_to_run = ticks;
        true
    }

    pub fn stop_stepping(&mut self) -> bool {
        if self.frozen_ticks_to_run <= 0 {
            return false;
        }
        self.frozen_ticks_to_run = 0;
        true
    }

    pub fn request_sprint(&mut self, ticks: i32) -> bool {
        let was_sprinting = self.remaining_sprint_ticks > 0;
        self.sprint_time_spent_nanos = 0;
        self.scheduled_sprint_ticks = i64::from(ticks);
        self.remaining_sprint_ticks = i64::from(ticks);
        self.previous_frozen = self.frozen;
        self.frozen = false;
        was_sprinting
    }

    pub fn stop_sprinting(&mut self) -> Option<SprintReport> {
        (self.remaining_sprint_ticks > 0).then(|| self.finish_sprint())
    }

    pub fn take_reports(&mut self) -> Vec<SprintReport> {
        std::mem::take(&mut self.reports)
    }

    pub fn query(&self) -> TickQuery {
        TickQuery {
            rate: self.rate,
            frozen: self.frozen,
            sprinting: self.is_sprinting(),
            nanos_per_tick: self.nanos_per_tick,
            average_nanos: self.average_nanos(),
            samples: self.samples,
        }
    }

    pub fn autosave_interval(&self) -> i32 {
        let rate = if self.is_sprinting() {
            NANOS_PER_SECOND as f32 / (self.average_nanos() + 1) as f32
        } else {
            self.rate
        };
        MIN_AUTOSAVE_TICKS.max((rate * AUTOSAVE_SECONDS) as i32)
    }

    pub fn ticks_until_autosave(&self) -> i32 {
        self.ticks_until_autosave
    }

    pub fn count_autosave(&mut self, ticks: usize) -> bool {
        if ticks == 0 {
            return false;
        }
        self.ticks_until_autosave -= ticks as i32;
        if self.ticks_until_autosave > 0 {
            return false;
        }
        self.ticks_until_autosave = self.autosave_interval();
        true
    }

    fn on_rate_changed(&mut self) {
        self.ticks_until_autosave = self.ticks_until_autosave.min(self.autosave_interval());
    }

    pub fn plan(&mut self, now: Instant, ready: bool) -> TickBatch {
        let mut batch = TickBatch::default();
        self.next_tick.get_or_insert(now);
        if !ready {
            return batch;
        }
        let limit = self.max_batch();
        while batch.len() < limit {
            if self.sprint_finishes_next() && !batch.is_empty() {
                break;
            }
            let nanos_per_tick = if self.is_sprinting() && self.check_should_sprint_this_tick() {
                self.next_tick = Some(now);
                self.last_overload_warning = Some(now);
                0
            } else {
                let next = self.next_tick.unwrap_or(now);
                if now < next {
                    break;
                }
                if let Some(overload) = self.check_overload(now, next) {
                    batch.overload = Some(overload);
                }
                self.nanos_per_tick
            };
            if let Some(next) = self.next_tick.as_mut() {
                *next += Duration::from_nanos(nanos_per_tick);
            }
            self.tick();
            batch.normal.push(self.run_game_elements);
            self.tick_count += 1;
        }
        batch
    }

    fn check_overload(&mut self, now: Instant, next: Instant) -> Option<(u64, u64)> {
        let nanos_per_tick = self.nanos_per_tick.max(1);
        let behind = now.duration_since(next).as_nanos() as u64;
        if behind <= OVERLOADED_THRESHOLD_NANOS + 20 * nanos_per_tick {
            return None;
        }
        let interval = OVERLOADED_WARNING_INTERVAL_NANOS + 100 * nanos_per_tick;
        if self
            .last_overload_warning
            .is_some_and(|last| (next.saturating_duration_since(last).as_nanos() as u64) < interval)
        {
            return None;
        }
        let ticks = behind / nanos_per_tick;
        let ms = behind / NANOS_PER_MILLISECOND;
        diag::warn!(
            World,
            "Can't keep up! Is the server overloaded? Running {}ms or {} ticks behind",
            ms,
            ticks
        );
        let skipped = next + Duration::from_nanos(ticks * nanos_per_tick);
        self.next_tick = Some(skipped);
        self.last_overload_warning = Some(skipped);
        Some((ms, ticks))
    }

    fn tick(&mut self) {
        self.run_game_elements = !self.frozen || self.frozen_ticks_to_run > 0;
        if self.frozen_ticks_to_run > 0 {
            self.frozen_ticks_to_run -= 1;
        }
    }

    fn sprint_finishes_next(&self) -> bool {
        self.is_sprinting() && self.run_game_elements && self.remaining_sprint_ticks <= 0
    }

    fn check_should_sprint_this_tick(&mut self) -> bool {
        if !self.run_game_elements {
            return false;
        }
        if self.remaining_sprint_ticks > 0 {
            self.remaining_sprint_ticks -= 1;
            return true;
        }
        let report = self.finish_sprint();
        self.reports.push(report);
        false
    }

    fn finish_sprint(&mut self) -> SprintReport {
        let done = self.scheduled_sprint_ticks - self.remaining_sprint_ticks;
        let ms = (self.sprint_time_spent_nanos as f64).max(1.0) / NANOS_PER_MILLISECOND as f64;
        let ticks_per_second = ((1000 * done) as f64 / ms) as i32;
        let ms_per_tick = if done == 0 {
            f64::from(self.nanos_per_tick as f32 / NANOS_PER_MILLISECOND as f32)
        } else {
            ms / done as f64
        };
        diag::info!(
            World,
            "Sprint completed with {} ticks per second, or {} ms per tick",
            ticks_per_second,
            minecraft_commands::tick::java_fixed(ms_per_tick, 2)
        );
        self.scheduled_sprint_ticks = 0;
        self.sprint_time_spent_nanos = 0;
        self.remaining_sprint_ticks = 0;
        self.frozen = self.previous_frozen;
        self.on_rate_changed();
        SprintReport {
            ticks_per_second,
            ms_per_tick,
        }
    }

    pub fn record(&mut self, nanos: &[u64]) {
        if nanos.is_empty() {
            return;
        }
        for &time in nanos {
            self.recorded += 1;
            let slot = (self.recorded % SAMPLES as u64) as usize;
            self.aggregated = self.aggregated - self.samples[slot] + time;
            self.samples[slot] = time;
            if self.is_sprinting() {
                self.sprint_time_spent_nanos += time;
            }
        }
        self.last_batch_nanos = (nanos.iter().sum::<u64>() / nanos.len() as u64).max(1);
    }

    pub fn average_nanos(&self) -> u64 {
        self.aggregated / self.recorded.clamp(1, SAMPLES as u64)
    }

    pub fn max_batch(&self) -> usize {
        (BATCH_BUDGET_NANOS / self.last_batch_nanos.max(1)).clamp(1, MAX_BATCH) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn calls_50_ms_apart_give_one_tick_each() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        for i in 0..20 {
            let now = base + ms(50 * i);
            let batch = rate.plan(now, true);
            assert_eq!(
                batch,
                TickBatch {
                    normal: vec![true],
                    overload: None
                },
                "call {i}"
            );
            rate.record(&[MS]);
            assert!(rate.plan(now, true).is_empty(), "call {i} again");
        }
        assert_eq!(rate.tick_count(), 20);
    }

    #[test]
    fn a_one_second_backlog_is_caught_up_in_batches_without_a_warning() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert_eq!(rate.plan(base, true).len(), 1);
        rate.record(&[10 * MS]);
        assert_eq!(rate.max_batch(), 5);
        let mut batches = Vec::new();
        loop {
            let batch = rate.plan(base + ms(1000), true);
            assert_eq!(batch.overload, None);
            if batch.is_empty() {
                break;
            }
            rate.record(&vec![10 * MS; batch.len()]);
            batches.push(batch.len());
        }
        assert_eq!(batches, [5, 5, 5, 5]);
    }

    #[test]
    fn the_batch_limit_follows_the_last_batch_not_the_average() {
        let mut rate = TickRate::default();
        assert_eq!(rate.max_batch(), 100);
        rate.record(&[1000 * MS; 100]);
        assert_eq!(rate.max_batch(), 1);
        rate.record(&[MS; 5]);
        assert_eq!(rate.max_batch(), 50);
        assert!(rate.average_nanos() > 900 * MS, "{}", rate.average_nanos());
    }

    #[test]
    fn three_seconds_behind_warns_once_and_skips() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert_eq!(rate.plan(base, true).len(), 1);
        let behind = rate.plan(base + ms(3000), true);
        assert_eq!(behind.overload, Some((3000 - 50, 59)));
        assert_eq!(behind.len(), 1);
        let again = rate.plan(base + ms(6000), true);
        assert_eq!(again.overload, None);
        assert_eq!(again.len(), 60);
        assert!(rate.plan(base + ms(6000), true).is_empty());
    }

    #[test]
    fn a_level_still_busy_plans_nothing_and_falls_behind() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert_eq!(rate.plan(base, true).len(), 1);
        assert_eq!(rate.plan(base + ms(3000), false), TickBatch::default());
        assert_eq!(rate.plan(base + ms(3000), true).overload, Some((2950, 59)));
    }

    #[test]
    fn the_first_plan_starts_the_schedule() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert!(rate.plan(base, false).is_empty());
        assert_eq!(rate.plan(base + ms(120), true).len(), 3);
    }

    #[test]
    fn the_average_covers_the_last_100_ticks() {
        let mut rate = TickRate::default();
        assert_eq!(rate.average_nanos(), 0);
        rate.record(&[2 * MS; 50]);
        assert_eq!(rate.average_nanos(), 2 * MS);
        rate.record(&[4 * MS; 50]);
        assert_eq!(rate.average_nanos(), 3 * MS);
        rate.record(&[4 * MS; 50]);
        assert_eq!(rate.average_nanos(), 4 * MS);
    }

    #[test]
    fn the_autosave_interval_is_five_minutes_at_the_tick_rate() {
        let mut rate = TickRate::default();
        assert_eq!(rate.autosave_interval(), 6000);
        rate.set_rate(1.0);
        assert_eq!(rate.autosave_interval(), 300);
        rate.set_rate(0.2);
        assert_eq!(rate.rate(), 1.0);
        rate.set_rate(10.0);
        assert_eq!(rate.autosave_interval(), 3000);
        rate.set_rate(0.3);
        assert_eq!(rate.autosave_interval(), 300);
    }

    #[test]
    fn the_autosave_interval_never_drops_below_100_ticks() {
        let mut rate = TickRate::default();
        rate.scheduled_sprint_ticks = 10;
        rate.record(&[10_000 * MS; 100]);
        assert_eq!(rate.autosave_interval(), 100);
        rate.scheduled_sprint_ticks = 0;
        rate.set_rate(MIN_RATE);
        assert_eq!(rate.autosave_interval(), 300);
    }

    #[test]
    fn lowering_the_rate_shortens_the_autosave_countdown_and_raising_it_does_not() {
        let mut rate = TickRate::default();
        assert_eq!(rate.ticks_until_autosave(), 6000);
        rate.set_rate(10.0);
        assert_eq!(rate.ticks_until_autosave(), 3000);
        rate.set_rate(40.0);
        assert_eq!(rate.ticks_until_autosave(), 3000);
        assert!(!rate.count_autosave(2999));
        assert!(rate.count_autosave(1));
        assert_eq!(rate.ticks_until_autosave(), 12000);
    }

    #[test]
    fn the_autosave_counts_down_by_whole_batches() {
        let mut rate = TickRate::default();
        assert!(!rate.count_autosave(0));
        assert!(!rate.count_autosave(5999));
        assert!(rate.count_autosave(100));
        assert_eq!(rate.ticks_until_autosave(), 6000);
    }

    #[test]
    fn the_client_ticks_at_the_rate_but_never_faster_than_20_a_second() {
        let mut rate = TickRate::default();
        assert_eq!(rate.client_timing(), ClientTiming::default());
        assert_eq!(rate.client_timing().tick_seconds, 0.05);
        rate.set_rate(5.0);
        assert_eq!(rate.client_timing().tick_seconds, 0.2);
        rate.set_rate(40.0);
        assert_eq!(rate.client_timing().tick_seconds, 0.05);
    }

    #[test]
    fn a_frozen_client_ticks_50_ms_and_holds_the_level_until_a_tick_runs_normally() {
        let mut rate = TickRate::default();
        rate.set_rate(5.0);
        let base = Instant::now();
        assert_eq!(rate.plan(base, true).normal, [true]);
        rate.set_frozen(true);
        assert!(
            rate.client_timing().runs_normally,
            "until a frozen tick is planned"
        );
        assert_eq!(rate.plan(base + ms(200), true).normal, [false]);
        let frozen = ClientTiming {
            tick_seconds: 0.05,
            runs_normally: false,
        };
        assert_eq!(rate.client_timing(), frozen);
        assert!(rate.plan(base + ms(300), true).is_empty());
        assert_eq!(
            rate.client_timing(),
            frozen,
            "a frame without ticks keeps it"
        );
        rate.set_frozen(false);
        assert_eq!(rate.plan(base + ms(400), true).normal, [true]);
        assert_eq!(rate.client_timing().tick_seconds, 0.2);
    }

    #[test]
    fn the_rate_is_at_least_one_tick_a_second() {
        let mut rate = TickRate::default();
        assert_eq!(rate.nanos_per_tick(), 50_000_000);
        rate.set_rate(1.0);
        assert_eq!(rate.nanos_per_tick(), 1_000_000_000);
        rate.set_rate(3.0);
        assert_eq!(rate.nanos_per_tick(), 333_333_333);
        rate.set_rate(10000.0);
        assert_eq!(rate.nanos_per_tick(), 100_000);
        rate.set_rate(0.5);
        assert_eq!((rate.rate(), rate.nanos_per_tick()), (1.0, 1_000_000_000));
        assert_eq!(rate.query().rate, 1.0);
        assert_eq!(rate.query().nanos_per_tick, 1_000_000_000);
    }

    #[test]
    fn steps_run_while_frozen_and_then_freeze_again() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert!(!rate.step(3), "only a frozen game steps");
        rate.set_frozen(true);
        assert!(rate.step(3));
        assert!(rate.is_stepping());
        let mut flags = Vec::new();
        for i in 0..5 {
            flags.extend(rate.plan(base + ms(50 * i), true).normal);
            rate.record(&[MS]);
        }
        assert_eq!(flags, [true, true, true, false, false]);
        assert!(!rate.is_stepping());
        assert!(!rate.stop_stepping());
        assert!(rate.step(100));
        assert!(rate.stop_stepping());
        assert!(!rate.stop_stepping());
        assert_eq!(rate.plan(base + ms(250), true).normal, [false]);
    }

    #[test]
    fn sprint_ticks_plan_without_waiting_and_record_their_time() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        assert!(!rate.request_sprint(10));
        assert!(rate.is_sprinting());
        assert!(rate.query().sprinting);
        let batch = rate.plan(base, true);
        assert_eq!(batch.normal, [true; 10]);
        assert!(
            rate.take_reports().is_empty(),
            "the last batch is still out"
        );
        rate.record(&[MS; 10]);
        assert_eq!(rate.plan(base, true).len(), 1);
        assert_eq!(
            rate.take_reports(),
            [SprintReport {
                ticks_per_second: 1000,
                ms_per_tick: 1.0
            }]
        );
        assert!(!rate.is_sprinting());
        assert!(rate.take_reports().is_empty());
    }

    #[test]
    fn a_sprint_reports_its_ticks_per_second_and_time_per_tick() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        rate.request_sprint(1200);
        let mut planned = 0;
        for frame in 0..100 {
            let batch = rate.plan(base + ms(frame), true);
            rate.record(&vec![MS / 2; batch.len()]);
            planned += batch.len();
            if !rate.is_sprinting() {
                break;
            }
        }
        assert_eq!(planned, 1201, "the sprint and one normal tick after it");
        assert_eq!(
            rate.take_reports(),
            [SprintReport {
                ticks_per_second: 2000,
                ms_per_tick: 0.5
            }]
        );
        rate.request_sprint(5);
        assert_eq!(
            rate.stop_sprinting(),
            Some(SprintReport {
                ticks_per_second: 0,
                ms_per_tick: 50.0
            })
        );
        assert!(
            rate.take_reports().is_empty(),
            "a stopped sprint answers the command"
        );
        assert_eq!(rate.stop_sprinting(), None);
    }

    #[test]
    fn a_sprint_gives_back_the_frozen_state_it_found() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        rate.set_frozen(true);
        assert_eq!(rate.plan(base, true).normal, [false]);
        assert!(!rate.request_sprint(3));
        assert!(!rate.is_frozen());
        let mut flags = Vec::new();
        for frame in 1..5 {
            let batch = rate.plan(base + ms(50 * frame), true);
            rate.record(&vec![MS; batch.len()]);
            flags.extend(batch.normal);
        }
        assert!(rate.is_frozen());
        assert_eq!(rate.take_reports().len(), 1);
        assert_eq!(flags.iter().filter(|&&normal| normal).count(), 4);
        assert_eq!(flags.last(), Some(&false));
    }

    #[test]
    fn freezing_while_sprinting_reports_once() {
        let mut rate = TickRate::default();
        let base = Instant::now();
        rate.request_sprint(200);
        assert_eq!(rate.plan(base, true).len(), 100);
        rate.record(&[MS; 100]);
        assert!(rate.stop_sprinting().is_some());
        rate.set_frozen(true);
        assert_eq!(rate.stop_sprinting(), None);
        rate.plan(base + ms(50), true);
        assert!(rate.take_reports().is_empty());
        assert!(rate.is_frozen());
    }

    #[test]
    fn a_batch_counts_its_normal_ticks() {
        let batch = TickBatch {
            normal: vec![true, false, true],
            overload: None,
        };
        assert_eq!(batch.len(), 3);
        assert_eq!(batch.normal_count(), 2);
    }
}
