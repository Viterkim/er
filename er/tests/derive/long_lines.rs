use er::*;
use std::fmt;

#[derive(Default, ErFormat)]
#[er(no_constructors)]
pub struct Wide {
    pub field0: u8,
    pub field1: u8,
    pub field2: u8,
    pub field3: u8,
    pub field4: u8,
    pub field5: u8,
    pub field6: u8,
    pub field7: u8,
    pub field8: u8,
    pub field9: u8,
    pub field10: u8,
    pub field11: u8,
    pub field12: u8,
    pub field13: u8,
    pub field14: u8,
    pub field15: u8,
    pub field16: u8,
    pub field17: u8,
    pub field18: u8,
    pub field19: u8,
    pub field20: u8,
    pub field21: u8,
    pub field22: u8,
    pub field23: u8,
    pub field24: u8,
    pub field25: u8,
    pub field26: u8,
    pub field27: u8,
    pub field28: u8,
    pub field29: u8,
    pub field30: u8,
    pub field31: u8,
    pub field32: u8,
    pub field33: u8,
    pub field34: u8,
    pub field35: u8,
    pub field36: u8,
    pub field37: u8,
    pub field38: u8,
    pub field39: u8,
    pub text: String,
}

#[derive(Er)]
pub struct WideErr {
    pub value: Wide,
}
#[test]
pub fn long_lines() -> fmt::Result {
    let value = Wide {
        text: "x".repeat(16 * 1024),
        ..Wide::default()
    };

    let mut expected = String::from("Wide { ");
    for index in 0..40 {
        expected.push_str(&format!("field{index}: 0, "));
    }
    expected.push_str(&format!("text: {:?} }}", value.text));

    let tree = ErTree::from(WideErr::new(value));
    let expected = format!("WideErr {{ value: {expected} }}");

    assert_eq!(tree.er_top().to_string(), expected);

    let report = tree.er_report().to_string();
    assert!(report.starts_with(&expected));

    let mut lines = Vec::new();
    tree.er_report()
        .for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, [report]);

    Ok(())
}
