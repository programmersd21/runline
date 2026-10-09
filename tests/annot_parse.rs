use runline::gh::{AnnotationKind, logs::parse_log_output};

#[test]
fn extracts_annotations_with_properties() {
    let raw = "j\tRun tests\t2026-01-01T00:00:00Z ::error file=src/main.rs,line=42,col=3::boom\n\
j\tRun tests\t2026-01-01T00:00:01Z ::warning file=a.rs,line=7::careful\n\
j\tRun tests\t2026-01-01T00:00:02Z ::notice::heads up\n\
j\tRun tests\t2026-01-01T00:00:03Z ##[error]plain bracket form\n\
j\tRun tests\t2026-01-01T00:00:04Z ##[command]/usr/bin/git init\n";
    let log = parse_log_output(1, raw);
    let anns: Vec<_> = log
        .lines
        .iter()
        .filter_map(|l| l.annotation.as_ref())
        .collect();
    println!("count = {}", anns.len());
    for a in &anns {
        println!(
            "{:?} msg={:?} file={:?} line={:?} col={:?} step={:?}",
            a.kind, a.message, a.file, a.line, a.col, a.step
        );
    }
    assert_eq!(anns.len(), 4, "only the four annotation commands count");
    assert_eq!(anns[0].kind, AnnotationKind::Error);
    assert_eq!(anns[0].file.as_deref(), Some("src/main.rs"));
    assert_eq!(anns[0].line, Some(42));
    assert_eq!(anns[0].col, Some(3));
    assert_eq!(anns[0].step.as_deref(), Some("Run tests"));
    assert_eq!(anns[1].kind, AnnotationKind::Warning);
    assert_eq!(anns[1].line, Some(7));
    assert_eq!(anns[2].kind, AnnotationKind::Notice);
    assert_eq!(anns[2].file, None);
    assert_eq!(anns[3].kind, AnnotationKind::Error);
}
