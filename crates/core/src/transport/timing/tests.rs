use super::*;

#[test]
fn a_connection_excludes_dns_from_connect_and_keeps_first_byte_cumulative() {
    let timings = ConnectionTimings::default();
    let started = Instant::now();
    let request = timings.begin(started);
    let dns = Duration::from_millis(10);
    timings
        .current()
        .unwrap()
        .record(Duration::from_millis(35), Some(dns));
    let first_byte = Duration::from_millis(50);
    let response = request.received_headers(started + first_byte);
    assert_eq!(response.dns, Some(dns));
    assert_eq!(response.connect, Some(Duration::from_millis(25)));
    assert_eq!(response.first_byte, first_byte);
}

#[test]
fn overlapping_requests_leave_connection_phases_unassigned() {
    let timings = ConnectionTimings::default();
    let started = Instant::now();
    let first = timings.begin(started);
    let pending_connection = timings.current().unwrap();
    let second = timings.begin(started);
    assert!(timings.current().is_none());

    pending_connection.record(Duration::from_millis(10), None);
    let headers = started + Duration::from_millis(20);
    assert_eq!(first.received_headers(headers).connect, None);
    assert_eq!(second.received_headers(headers).connect, None);
    drop(first);
    assert!(timings.current().is_none());
    drop(second);

    let next = timings.begin(started);
    assert!(timings.current().is_some());
    assert_eq!(next.received_headers(headers).connect, None);
}

#[test]
fn a_late_connection_cannot_add_timings_to_a_later_request() {
    let timings = ConnectionTimings::default();
    let started = Instant::now();
    let first = timings.begin(started);
    let pending_connection = timings.current().unwrap();
    let headers = started + Duration::from_millis(20);
    assert_eq!(first.received_headers(headers).connect, None);
    drop(first);

    let next = timings.begin(headers);
    pending_connection.record(Duration::from_millis(50), None);
    assert_eq!(next.received_headers(headers).connect, None);
}
