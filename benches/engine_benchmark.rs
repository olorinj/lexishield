use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use lexishield::config::LexiConfig;
use lexishield::engine::ObfuscatorEngine;
use lexishield::models::{DetectorType, FormatType, Mapping};

fn generate_synthetic_log(lines_count: usize) -> String {
    let mut log = String::with_capacity(lines_count * 150);
    for i in 0..lines_count {
        log.push_str(&format!(
            "2026-10-04T12:00:{:02}.000Z [INFO] Host srv-prod{:02} IP 192.168.1.{} User admin{}@corp.es Session 0x{:08X} DNI 12345678Z GUID c9a646d3-9c61-4cb7-897d-4b958c218a56\n",
            i % 60,
            i % 10,
            i % 250,
            i % 50,
            0x1000 + i
        ));
    }
    log
}

fn bench_scanning(c: &mut Criterion) {
    let mut group = c.benchmark_group("Engine_Scanning");
    let log_100_lines = generate_synthetic_log(100);
    let log_1000_lines = generate_synthetic_log(1000);

    group.throughput(Throughput::Bytes(log_100_lines.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("scan_log", "100_lines"),
        &log_100_lines,
        |b, input| {
            b.iter(|| {
                let config = LexiConfig::default();
                let mut engine = ObfuscatorEngine::new(config);
                let _ = engine.scan_and_register_mappings(black_box(input));
            });
        },
    );

    group.throughput(Throughput::Bytes(log_1000_lines.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("scan_log", "1000_lines"),
        &log_1000_lines,
        |b, input| {
            b.iter(|| {
                let config = LexiConfig::default();
                let mut engine = ObfuscatorEngine::new(config);
                let _ = engine.scan_and_register_mappings(black_box(input));
            });
        },
    );
    group.finish();
}

fn bench_obfuscation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Engine_Obfuscation");
    let log = generate_synthetic_log(500);

    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);
    let _ = engine.scan_and_register_mappings(&log);

    group.throughput(Throughput::Bytes(log.len() as u64));
    group.bench_function("obfuscate_plaintext_500_lines", |b| {
        b.iter(|| {
            let (result, _) = engine
                .obfuscate_text(black_box(&log), FormatType::Plaintext)
                .unwrap();
            black_box(result);
        });
    });

    let json_payload = r#"{
        "server_ip": "192.168.1.100",
        "admin_email": "admin@corp.es",
        "dni": "12345678Z",
        "nested": {
            "backup_server": "10.0.0.1",
            "session_guid": "c9a646d3-9c61-4cb7-897d-4b958c218a56"
        }
    }"#;

    let mut json_engine = ObfuscatorEngine::new(LexiConfig::default());
    json_engine
        .manager
        .add_mapping(Mapping::new(
            "192.168.1.100",
            "192.0.2.1",
            DetectorType::IPv4,
        ))
        .unwrap();
    json_engine
        .manager
        .add_mapping(Mapping::new(
            "admin@corp.es",
            "user_01@example.com",
            DetectorType::Email,
        ))
        .unwrap();
    json_engine
        .manager
        .add_mapping(Mapping::new(
            "12345678Z",
            "87654321X",
            DetectorType::SpanishDniNie,
        ))
        .unwrap();

    group.throughput(Throughput::Bytes(json_payload.len() as u64));
    group.bench_function("obfuscate_json_structured", |b| {
        b.iter(|| {
            let (result, _) = json_engine
                .obfuscate_text(black_box(json_payload), FormatType::Json)
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scanning, bench_obfuscation);
criterion_main!(benches);
