#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_uncached_vs_cached() {
        // Test data with various Unicode characters
        let test_strings = vec![
            "Simple ASCII text",
            "Café with accents",
            "中文 Chinese characters",
            "🦀 Rust emoji 🚀",
            "Mixed: Hello 世界 🌍",
            "Very long string that needs truncation and has Unicode characters like café and 中文",
            "Artist Name - Album Title (2023)",
            "01. Song Title with Special Characters: ♫ ♪ ♬",
        ];

        // Benchmark uncached version
        let start = Instant::now();
        let mut total_width_uncached = 0;
        for _ in 0..1000 {
            for s in &test_strings {
                total_width_uncached += s.width();
            }
        }
        let uncached_duration = start.elapsed();

        // Benchmark cached version
        let mut cache = WidthCache::new();
        let start = Instant::now();
        let mut total_width_cached = 0;
        for _ in 0..1000 {
            for s in &test_strings {
                total_width_cached += cache.get_width(s);
            }
        }
        let cached_duration = start.elapsed();

        // Verify results are the same
        assert_eq!(total_width_uncached, total_width_cached);

        // Report performance improvement
        let improvement = uncached_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64;
        let hit_rate = cache.hit_rate();

        println!("Performance Benchmark Results:");
        println!("  Uncached: {:?}", uncached_duration);
        println!("  Cached:   {:?}", cached_duration);
        println!("  Speedup:  {:.2}x", improvement);
        println!("  Hit rate: {:.1}%", hit_rate * 100.0);
        println!("  Cache entries: {}", cache.len());

        // Cache should have all test strings after first iteration
        assert!(hit_rate > 0.99, "Hit rate should be very high after warm-up");
        assert!(improvement > 2.0, "Should see at least 2x improvement");
    }

    #[test]
    fn bench_truncation_performance() {
        use crate::ui::utils::{truncate_by_width, truncate_by_width_cached};
        
        let test_strings = vec![
            "This is a very long string that will need truncation",
            "Short",
            "Mixed Unicode: Hello 世界 🌍 with emoji",
            "Artist Name - Very Long Album Title (Special Edition)",
        ];

        let max_width = 20;
        let iterations = 500;

        // Benchmark uncached truncation
        let start = Instant::now();
        for _ in 0..iterations {
            for s in &test_strings {
                let _ = truncate_by_width(s, max_width);
            }
        }
        let uncached_duration = start.elapsed();

        // Benchmark cached truncation
        let mut cache = WidthCache::new();
        let start = Instant::now();
        for _ in 0..iterations {
            for s in &test_strings {
                let _ = truncate_by_width_cached(&mut cache, s, max_width);
            }
        }
        let cached_duration = start.elapsed();

        let improvement = uncached_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64;

        println!("Truncation Benchmark Results:");
        println!("  Uncached: {:?}", uncached_duration);
        println!("  Cached:   {:?}", cached_duration);
        println!("  Speedup:  {:.2}x", improvement);

        assert!(improvement > 1.5, "Should see significant improvement in truncation");
    }
}