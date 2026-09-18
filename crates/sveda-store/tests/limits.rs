use std::time::Duration;

use sveda_store::{
    parse_laravel_throttle, parse_optional_laravel_throttle, Occupancy, OccupancyError,
    RateLimiter, OCC_GLOBAL_KEY, OCC_VISITOR_PREFIX, RL_PREFIX,
};

#[test]
fn parse_embed_throttle_matches_laravel_decay_minutes() {
    assert_eq!(parse_laravel_throttle("30,1"), (30, 60));
    assert_eq!(parse_laravel_throttle("10,2"), (10, 120));
    assert_eq!(parse_laravel_throttle(""), (30, 60));
    assert_eq!(parse_laravel_throttle("0,1"), (0, 60));
    assert_eq!(parse_laravel_throttle("0"), (0, 60));
}

#[test]
fn parse_optional_throttle_defaults_off() {
    assert_eq!(parse_optional_laravel_throttle(""), (0, 60));
    assert_eq!(parse_optional_laravel_throttle("20,2"), (20, 120));
    assert_eq!(parse_optional_laravel_throttle("0,1"), (0, 60));
}

#[test]
fn occupancy_keys_are_redis_shaped() {
    assert_eq!(OCC_GLOBAL_KEY, "sveda:occ:global");
    assert_eq!(
        Occupancy::visitor_key("abc"),
        format!("{OCC_VISITOR_PREFIX}abc")
    );
    assert_eq!(
        RateLimiter::throttle_key("embed"),
        format!("{RL_PREFIX}embed")
    );
}

#[test]
fn occupancy_global_limit_releases_on_drop() {
    let occupancy = Occupancy::memory(1, 0);
    let first = occupancy.acquire("a").expect("first");
    assert_eq!(occupancy.acquire("b").unwrap_err(), OccupancyError::Global);
    drop(first);
    assert!(occupancy.acquire("b").is_ok());
}

#[test]
fn occupancy_per_visitor_does_not_block_other_visitors() {
    let occupancy = Occupancy::memory(10, 1);
    let _held = occupancy.acquire("a").expect("a");
    assert_eq!(occupancy.acquire("a").unwrap_err(), OccupancyError::Visitor);
    assert!(occupancy.acquire("b").is_ok());
}

#[test]
fn unlimited_occupancy_always_acquires() {
    let occupancy = Occupancy::memory(0, 0);
    let _a = occupancy.acquire("a").unwrap();
    let _b = occupancy.acquire("a").unwrap();
    let _c = occupancy.acquire("b").unwrap();
}

#[test]
fn rate_limiter_blocks_after_budget() {
    let limiter = RateLimiter::memory(2, Duration::from_secs(60));
    assert!(limiter.hit("embed").is_ok());
    assert!(limiter.hit("embed").is_ok());
    let retry = limiter.hit("embed").unwrap_err();
    assert!(retry >= 1);
    assert!(limiter.hit("other").is_ok());
}

#[test]
fn rate_limiter_set_limits_applies_to_clones() {
    let limiter = RateLimiter::memory(10, Duration::from_secs(60));
    let clone = limiter.clone();
    limiter.set_limits(1, Duration::from_secs(60));
    assert!(clone.hit("k").is_ok());
    assert!(clone.hit("k").is_err());
}

#[test]
fn occupancy_set_limits_enables_and_disables() {
    let occupancy = Occupancy::memory(0, 0);
    let _a = occupancy.acquire("a").unwrap();
    let _b = occupancy.acquire("a").unwrap();
    occupancy.set_limits(1, 0);
    let held = occupancy.acquire("c").unwrap();
    assert_eq!(occupancy.acquire("d").unwrap_err(), OccupancyError::Global);
    drop(held);
    occupancy.set_limits(0, 0);
    assert!(occupancy.acquire("d").is_ok());
}
