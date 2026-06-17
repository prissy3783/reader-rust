use crate::model::book_source::BookSource;

#[derive(Debug, Clone)]
pub struct SourceHealth {
    pub level: i32,
    pub latency_ms: i64,
    pub search_ok: bool,
    pub toc_ok: bool,
    pub content_ok: bool,
    pub consecutive_failures: i32,
}

impl SourceHealth {
    pub fn score(&self) -> f64 {
        let mut score: f64 = 0.0;

        if self.search_ok {
            score += 30.0;
        }
        if self.toc_ok {
            score += 30.0;
        }
        if self.content_ok {
            score += 30.0;
        }

        let latency_score = match self.latency_ms {
            0..=1000 => 10.0,
            1001..=3000 => 7.0,
            3001..=5000 => 4.0,
            5001..=10000 => 2.0,
            _ => 0.0,
        };
        score += latency_score;

        let failure_penalty = (self.consecutive_failures as f64) * 5.0;
        score -= failure_penalty;

        score.clamp(0.0, 100.0)
    }

    pub fn level_from_score(score: f64) -> i32 {
        match score as i64 {
            80..=100 => 5,
            60..=79 => 4,
            40..=59 => 3,
            20..=39 => 2,
            _ => 1,
        }
    }

    pub fn compute_level(&self) -> i32 {
        Self::level_from_score(self.score())
    }
}

pub fn apply_health_to_source(source: &mut BookSource, health: &SourceHealth) {
    source.health_level = Some(health.compute_level() as i64);
    source.latency_ms = Some(health.latency_ms);
    source.last_check_time = Some(crate::util::time::now_ts());
    source.consecutive_failures = Some(health.consecutive_failures as i64);
}

pub fn sort_sources_by_health(sources: &mut [BookSource]) {
    sources.sort_by(|a, b| {
        let level_a = a.health_level.unwrap_or(0);
        let level_b = b.health_level.unwrap_or(0);
        let latency_a = a.latency_ms.unwrap_or(i64::MAX);
        let latency_b = b.latency_ms.unwrap_or(i64::MAX);

        level_b
            .cmp(&level_a)
            .then_with(|| latency_a.cmp(&latency_b))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_health(
        search_ok: bool,
        toc_ok: bool,
        content_ok: bool,
        latency_ms: i64,
        failures: i32,
    ) -> SourceHealth {
        SourceHealth {
            level: 0,
            latency_ms,
            search_ok,
            toc_ok,
            content_ok,
            consecutive_failures: failures,
        }
    }

    #[test]
    fn test_perfect_source() {
        let h = make_health(true, true, true, 500, 0);
        assert_eq!(h.score(), 100.0);
        assert_eq!(h.compute_level(), 5);
    }

    #[test]
    fn test_search_only() {
        let h = make_health(true, false, false, 1000, 0);
        let score = h.score();
        assert!(score > 30.0 && score < 50.0);
        assert_eq!(h.compute_level(), 3);
    }

    #[test]
    fn test_slow_source() {
        let h = make_health(true, true, true, 8000, 0);
        let score = h.score();
        assert!(score < 100.0);
        assert!(h.compute_level() >= 4);
    }

    #[test]
    fn test_failed_source() {
        let h = make_health(false, false, false, 5000, 3);
        let score = h.score();
        assert_eq!(score, 0.0);
        assert_eq!(h.compute_level(), 1);
    }

    #[test]
    fn test_level_boundaries() {
        assert_eq!(SourceHealth::level_from_score(100.0), 5);
        assert_eq!(SourceHealth::level_from_score(80.0), 5);
        assert_eq!(SourceHealth::level_from_score(79.0), 4);
        assert_eq!(SourceHealth::level_from_score(60.0), 4);
        assert_eq!(SourceHealth::level_from_score(59.0), 3);
        assert_eq!(SourceHealth::level_from_score(40.0), 3);
        assert_eq!(SourceHealth::level_from_score(39.0), 2);
        assert_eq!(SourceHealth::level_from_score(20.0), 2);
        assert_eq!(SourceHealth::level_from_score(19.0), 1);
        assert_eq!(SourceHealth::level_from_score(0.0), 1);
    }

    #[test]
    fn test_sort_by_health() {
        let mut sources = vec![
            {
                let mut s = BookSource::default();
                s.health_level = Some(2);
                s.latency_ms = Some(3000);
                s
            },
            {
                let mut s = BookSource::default();
                s.health_level = Some(5);
                s.latency_ms = Some(500);
                s
            },
            {
                let mut s = BookSource::default();
                s.health_level = Some(5);
                s.latency_ms = Some(200);
                s
            },
        ];
        sort_sources_by_health(&mut sources);
        assert_eq!(sources[0].latency_ms, Some(200));
        assert_eq!(sources[1].latency_ms, Some(500));
        assert_eq!(sources[2].health_level, Some(2));
    }
}
